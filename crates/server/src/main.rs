use sentinel_server::broker::InMemoryBroker;
use sentinel_server::config::ServerConfig;
use sentinel_server::grpc::AgentServiceImpl;
use sentinel_server::rest::{self, AppState};
use sentinel_server::store::{AgentStore, IdempotencyStore, RuleStore};
use sentinel_server::tls::TlsIdentity;

use sentinel_common::proto::agent_service_server::AgentServiceServer;
use tonic::transport::Server as TonicServer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = ServerConfig::default();
    let agents = AgentStore::new();
    let idempotency = IdempotencyStore::new();
    let broker = InMemoryBroker::new();

    let tls_identity = config.tls.as_ref().map(|tls_cfg| {
        TlsIdentity::load(tls_cfg).expect("failed to load TLS certificates")
    });

    let grpc_service = AgentServiceImpl::new(agents.clone(), idempotency, broker);
    let grpc_addr = config.grpc_addr;

    let grpc_tls = tls_identity
        .as_ref()
        .map(|id| id.tonic_server_tls().expect("failed to build gRPC TLS config"));

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
