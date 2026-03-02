use std::sync::Arc;

use sentinel_server::broker::{connect_jetstream, ensure_stream, NatsPublisher};
use sentinel_server::config::ServerConfig;
use sentinel_server::grpc::AgentServiceImpl;
use sentinel_server::metrics::server_metrics::ServerMetrics;
use sentinel_server::migration;
use sentinel_server::persistence::AgentRepo;
use sentinel_server::provisioning::TokenStore;
use sentinel_server::rest::{self, AppState};
use sentinel_server::store::{AgentStore, IdempotencyStore, RuleStore};
use sentinel_server::stream::{
    spawn_watchdog, PresenceEventBus, SessionRegistry, StreamService, WatchdogConfig,
};
use sentinel_server::tls::TlsIdentity;

use sentinel_common::nats_config::StreamConfig;
use sentinel_common::proto::agent_service_server::AgentServiceServer;
use sentinel_common::proto::sentinel_stream_server::SentinelStreamServer;
use tonic::transport::Server as TonicServer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .json()
        .init();

    tracing::info!("SentinelRS Server v{} starting", env!("CARGO_PKG_VERSION"));

    let config = ServerConfig::from_env_and_args();

    let (pool, agent_repo) = match config.database_url {
        Some(ref url) => {
            tracing::info!("waiting for database...");

            let health_config = migration::HealthConfig::default();
            let pool = migration::wait_for_db(url, 10, &health_config)
                .await
                .unwrap_or_else(|e| {
                    tracing::error!(error = %e, "database health check failed");
                    std::process::exit(1);
                });
            tracing::info!("database connection established");

            match migration::run(&pool).await {
                Ok(0) => tracing::info!("database schema up to date"),
                Ok(n) => tracing::info!(count = n, "auto-migration completed successfully"),
                Err(e) => {
                    tracing::error!(error = %e, "auto-migration failed");
                    std::process::exit(1);
                }
            }

            let repo = Arc::new(AgentRepo::new(pool.clone()));
            (Some(pool), Some(repo))
        }
        None => {
            tracing::warn!("no DATABASE_URL configured, agent store is in-memory only");
            (None, None)
        }
    };

    let js = connect_jetstream(&config.nats_url)
        .await
        .expect("failed to connect to NATS JetStream");
    tracing::info!(url = %config.nats_url, "connected to NATS JetStream");

    let stream_config = StreamConfig::default();
    ensure_stream(&js, &stream_config)
        .await
        .expect("failed to ensure NATS stream");
    tracing::info!(stream = %stream_config.name, "NATS stream ready");

    let agents = AgentStore::new();
    let server_metrics = ServerMetrics::new();

    if let Some(ref repo) = agent_repo {
        match repo.load_all(&agents).await {
            Ok(n) => tracing::info!(count = n, "loaded agents from database"),
            Err(e) => tracing::error!(error = %e, "failed to load agents from database"),
        }
    }

    let tls_identity = config
        .tls
        .as_ref()
        .map(|tls_cfg| TlsIdentity::load(tls_cfg).expect("failed to load TLS certificates"));

    let idempotency = IdempotencyStore::new();
    let broker = Arc::new(NatsPublisher::new(js));

    let grpc_service = {
        let svc = AgentServiceImpl::new(agents.clone(), idempotency.clone(), broker.clone());
        match agent_repo {
            Some(ref repo) => svc.with_repo(repo.clone()),
            None => svc,
        }
    };

    let session_registry = SessionRegistry::new();
    let presence_events = PresenceEventBus::new();
    let token_store = TokenStore::new();
    let grpc_public_url = format!("http://{}", config.grpc_addr);

    spawn_watchdog(
        session_registry.clone(),
        presence_events.clone(),
        WatchdogConfig::default(),
    );

    let stream_service = StreamService::new(
        agents.clone(),
        idempotency,
        broker,
        session_registry.clone(),
        presence_events.clone(),
        config.key_grace_period_ms,
    )
    .with_provisioning(
        token_store.clone(),
        agent_repo.clone(),
        grpc_public_url.clone(),
    );

    let grpc_addr = config.grpc_addr;

    let grpc_tls = tls_identity.as_ref().map(|id| {
        id.tonic_server_tls()
            .expect("failed to build gRPC TLS config")
    });

    let grpc_handle = tokio::spawn(async move {
        tracing::info!(%grpc_addr, "gRPC listening (V1 + V2 streaming)");
        let mut builder = TonicServer::builder();
        if let Some(tls) = grpc_tls {
            builder = builder.tls_config(tls).expect("invalid gRPC TLS config");
        }
        builder
            .add_service(AgentServiceServer::new(grpc_service))
            .add_service(SentinelStreamServer::new(stream_service))
            .serve(grpc_addr)
            .await
            .expect("gRPC server failed");
    });

    let app_state = AppState {
        agents,
        rules: RuleStore::new(),
        jwt_secret: config.jwt_secret,
        metrics: server_metrics,
        pool,
        token_store: Some(token_store),
        grpc_public_url,
        registry: session_registry,
        events: presence_events,
    };
    let rest_app = rest::router(app_state);
    let rest_addr = config.rest_addr;

    let rest_handle = tokio::spawn(async move {
        tracing::info!(%rest_addr, "REST listening");
        let listener = tokio::net::TcpListener::bind(rest_addr).await.unwrap();
        axum::serve(listener, rest_app).await.unwrap();
    });

    tokio::select! {
        r = grpc_handle => { if let Err(e) = r { tracing::error!("gRPC: {e}"); } }
        r = rest_handle => { if let Err(e) = r { tracing::error!("REST: {e}"); } }
    }
}
