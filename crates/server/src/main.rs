use std::sync::Arc;

use sentinel_common::logging::{self, Component, LogConfig};
use sentinel_common::nats_config::StreamConfig;
use sentinel_common::proto::agent_service_server::AgentServiceServer;
use sentinel_common::proto::sentinel_stream_server::SentinelStreamServer;

use sentinel_server::broker::{connect_jetstream, ensure_stream, NatsPublisher};
use sentinel_server::config::ServerConfig;
use sentinel_server::grpc::AgentServiceImpl;
use sentinel_server::metrics::server_metrics::ServerMetrics;
use sentinel_server::migration;
use sentinel_server::persistence::{
    AgentRepo, MetricsQueryRepo, NotificationHistoryRepo, NotifierRepo, RuleRepo,
};
use sentinel_server::provisioning::TokenStore;
use sentinel_server::rest::{self, AppState};
use sentinel_server::store::{AgentStore, IdempotencyStore, RuleStore};
use sentinel_server::stream::{
    spawn_watchdog, PresenceEventBus, SessionRegistry, StreamService, WatchdogConfig,
};
use sentinel_server::tls::TlsIdentity;

use tonic::transport::Server as TonicServer;

#[tokio::main]
async fn main() {
    let log_config = LogConfig::from_env();
    logging::print_banner(Component::Server, env!("CARGO_PKG_VERSION"));
    logging::init(&log_config);

    tracing::info!(target: "system", "Starting SentinelRS Server v{}", env!("CARGO_PKG_VERSION"));

    let config = ServerConfig::from_env_and_args();

    let (pool, agent_repo, rule_repo, notifier_repo, history_repo, metrics_qrepo) =
        match config.database_url {
            Some(ref url) => {
                let sw = logging::stopwatch();
                let health_config = migration::HealthConfig::default();
                let pool = migration::wait_for_db(url, 10, &health_config)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!(target: "db", error = %e, "Database health check failed");
                        std::process::exit(1);
                    });
                tracing::info!(target: "db", "Database connected{sw}");

                match migration::run(&pool).await {
                    Ok(0) => tracing::info!(target: "db", "Schema up to date"),
                    Ok(n) => tracing::info!(target: "db", count = n, "Auto-migration completed"),
                    Err(e) => {
                        tracing::error!(target: "db", error = %e, "Auto-migration failed");
                        std::process::exit(1);
                    }
                }

                let repo = Arc::new(AgentRepo::new(pool.clone()));
                let r_repo = Arc::new(RuleRepo::new(pool.clone()));
                let n_repo = Arc::new(NotifierRepo::new(pool.clone()));
                let h_repo = Arc::new(NotificationHistoryRepo::new(pool.clone()));
                let m_repo = Arc::new(MetricsQueryRepo::new(pool.clone()));
                (
                    Some(pool),
                    Some(repo),
                    Some(r_repo),
                    Some(n_repo),
                    Some(h_repo),
                    Some(m_repo),
                )
            }
            None => {
                tracing::warn!(target: "db", "No DATABASE_URL — in-memory mode");
                (None, None, None, None, None, None)
            }
        };

    let sw = logging::stopwatch();
    let js = connect_jetstream(&config.nats_url)
        .await
        .expect("NATS JetStream connection failed");
    tracing::info!(target: "net", "NATS JetStream connected ({}){sw}", config.nats_url);

    let stream_config = StreamConfig::default();
    ensure_stream(&js, &stream_config)
        .await
        .expect("NATS stream creation failed");
    tracing::info!(target: "net", "Stream '{}' ready", stream_config.name);

    let agents = AgentStore::new();
    let rules = RuleStore::new();
    let server_metrics = ServerMetrics::new();

    if let Some(ref repo) = agent_repo {
        match repo.load_all(&agents).await {
            Ok(n) => tracing::info!(target: "data", count = n, "Loaded agents from database"),
            Err(e) => tracing::error!(target: "data", error = %e, "Failed to load agents"),
        }
    }

    if let Some(ref repo) = rule_repo {
        match repo.load_all(&rules).await {
            Ok(n) => tracing::info!(target: "data", count = n, "Loaded alert rules from database"),
            Err(e) => tracing::error!(target: "data", error = %e, "Failed to load alert rules"),
        }
    }

    let tls_identity = config
        .tls
        .as_ref()
        .map(|tls_cfg| TlsIdentity::load(tls_cfg).expect("TLS certificate load failed"));

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
    let grpc_public_url = config
        .grpc_advertise_addr
        .clone()
        .unwrap_or_else(|| format!("http://{}", config.grpc_addr));

    spawn_watchdog(
        session_registry.clone(),
        presence_events.clone(),
        WatchdogConfig::default(),
    );
    tracing::info!(target: "conn", "Watchdog active");

    let stream_service = StreamService::new(
        agents.clone(),
        idempotency,
        broker,
        session_registry.clone(),
        presence_events.clone(),
        config.key_grace_period_ms,
        server_metrics.clone(),
    )
    .with_provisioning(
        token_store.clone(),
        agent_repo.clone(),
        grpc_public_url.clone(),
    );

    let grpc_addr = config.grpc_addr;

    let grpc_tls = tls_identity
        .as_ref()
        .map(|id| id.tonic_server_tls().expect("gRPC TLS config build failed"));

    let grpc_handle = tokio::spawn(async move {
        tracing::info!(target: "net", "gRPC listening on {grpc_addr} (V1 + V2 streaming)");
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
        rules,
        rule_repo,
        notifier_repo,
        history_repo,
        metrics_repo: metrics_qrepo,
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
        tracing::info!(target: "net", "REST API listening on {rest_addr}");
        let listener = tokio::net::TcpListener::bind(rest_addr).await.unwrap();
        axum::serve(listener, rest_app).await.unwrap();
    });

    tracing::info!(target: "system", "Server ready");

    tokio::select! {
        r = grpc_handle => { if let Err(e) = r { tracing::error!(target: "net", "gRPC: {e}"); } }
        r = rest_handle => { if let Err(e) = r { tracing::error!(target: "net", "REST: {e}"); } }
    }
}
