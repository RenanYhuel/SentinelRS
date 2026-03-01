use std::sync::Arc;

use sentinel_server::broker::{connect_jetstream, ensure_stream, NatsPublisher};
use sentinel_server::config::ServerConfig;
use sentinel_server::grpc::AgentServiceImpl;
use sentinel_server::metrics::server_metrics::ServerMetrics;
use sentinel_server::persistence::{create_pool, AgentRepo};
use sentinel_server::rest::{self, AppState};
use sentinel_server::store::{AgentStore, IdempotencyStore, RuleStore};
use sentinel_server::tls::TlsIdentity;

use sentinel_common::nats_config::StreamConfig;
use sentinel_common::proto::agent_service_server::AgentServiceServer;
use tonic::transport::Server as TonicServer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .json()
        .init();

    let config = ServerConfig::from_env_and_args();

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
    let idempotency = IdempotencyStore::new();
    let broker = NatsPublisher::new(js);
    let server_metrics = ServerMetrics::new();

    let agent_repo = match config.database_url {
        Some(ref url) => {
            let pool = create_pool(url, 10)
                .await
                .expect("failed to connect to PostgreSQL");
            tracing::info!("connected to PostgreSQL");

            let repo = Arc::new(AgentRepo::new(pool));
            match repo.load_all(&agents).await {
                Ok(n) => tracing::info!(count = n, "loaded agents from database"),
                Err(e) => tracing::error!(error = %e, "failed to load agents from database"),
            }
            Some(repo)
        }
        None => {
            tracing::warn!("no DATABASE_URL configured, agent store is in-memory only");
            None
        }
    };

    let db_pool = agent_repo.as_ref().map(|r| r.pool().clone());

    let tls_identity = config
        .tls
        .as_ref()
        .map(|tls_cfg| TlsIdentity::load(tls_cfg).expect("failed to load TLS certificates"));

    let grpc_service = {
        let svc = AgentServiceImpl::new(agents.clone(), idempotency, broker);
        match agent_repo {
            Some(repo) => svc.with_repo(repo),
            None => svc,
        }
    };
    let grpc_addr = config.grpc_addr;

    let grpc_tls = tls_identity.as_ref().map(|id| {
        id.tonic_server_tls()
            .expect("failed to build gRPC TLS config")
    });

    let grpc_handle = tokio::spawn(async move {
        tracing::info!(%grpc_addr, "gRPC server starting");
        let mut builder = TonicServer::builder();
        if let Some(tls) = grpc_tls {
            builder = builder.tls_config(tls).expect("invalid gRPC TLS config");
        }
        builder
            .add_service(AgentServiceServer::new(grpc_service))
            .serve(grpc_addr)
            .await
            .expect("gRPC server failed");
    });

    let app_state = AppState {
        agents,
        rules: RuleStore::new(),
        jwt_secret: config.jwt_secret,
        metrics: server_metrics,
        pool: db_pool,
    };
    let rest_app = rest::router(app_state);
    let rest_addr = config.rest_addr;

    let rest_handle = tokio::spawn(async move {
        tracing::info!(%rest_addr, "REST server starting");
        let listener = tokio::net::TcpListener::bind(rest_addr).await.unwrap();
        axum::serve(listener, rest_app).await.unwrap();
    });

    tokio::select! {
        r = grpc_handle => { if let Err(e) = r { tracing::error!("gRPC: {e}"); } }
        r = rest_handle => { if let Err(e) = r { tracing::error!("REST: {e}"); } }
    }
}
