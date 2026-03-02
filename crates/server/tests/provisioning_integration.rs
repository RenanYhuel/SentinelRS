use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{Duration, Utc};

use sentinel_common::proto::agent_message::Payload as AgentPayload;
use sentinel_common::proto::server_message::Payload as ServerPayload;
use sentinel_common::proto::sentinel_stream_client::SentinelStreamClient;
use sentinel_common::proto::sentinel_stream_server::SentinelStreamServer;
use sentinel_common::proto::{AgentMessage, BootstrapRequest, BootstrapStatus};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Channel, Server};

use sentinel_server::broker::InMemoryBroker;
use sentinel_server::provisioning::{BootstrapToken, TokenStore};
use sentinel_server::store::{AgentStore, IdempotencyStore};
use sentinel_server::stream::{PresenceEventBus, SessionRegistry, StreamService};

struct ProvisioningTestServer {
    addr: std::net::SocketAddr,
    agents: AgentStore,
    token_store: TokenStore,
    shutdown: Arc<AtomicBool>,
}

impl ProvisioningTestServer {
    async fn start() -> Self {
        let agents = AgentStore::new();
        let idempotency = IdempotencyStore::new();
        let broker = Arc::new(InMemoryBroker::new());
        let registry = SessionRegistry::new();
        let events = PresenceEventBus::new();
        let token_store = TokenStore::new();
        let shutdown = Arc::new(AtomicBool::new(false));

        let svc = StreamService::new(
            agents.clone(),
            idempotency,
            broker,
            registry,
            events,
            300_000,
        )
        .with_provisioning(token_store.clone(), None, "grpc://127.0.0.1:50051".into());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let shutdown_flag = shutdown.clone();
        tokio::spawn(async move {
            let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
            Server::builder()
                .add_service(SentinelStreamServer::new(svc))
                .serve_with_incoming_shutdown(incoming, async move {
                    while !shutdown_flag.load(Ordering::Relaxed) {
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Self {
            addr,
            agents,
            token_store,
            shutdown,
        }
    }

    async fn client(&self) -> SentinelStreamClient<Channel> {
        let endpoint = format!("http://{}", self.addr);
        let channel = Channel::from_shared(endpoint)
            .unwrap()
            .connect()
            .await
            .unwrap();
        SentinelStreamClient::new(channel)
    }

    fn insert_token(&self, token: &str, agent_name: &str) {
        self.token_store.insert(BootstrapToken {
            token: token.into(),
            agent_name: agent_name.into(),
            labels: vec![],
            created_by: "test".into(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(1),
            consumed: false,
        });
    }
}

impl Drop for ProvisioningTestServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

fn bootstrap_message(token: &str) -> AgentMessage {
    AgentMessage {
        payload: Some(AgentPayload::BootstrapRequest(BootstrapRequest {
            bootstrap_token: token.into(),
            hw_id: "hw-test-001".into(),
            agent_version: "1.0.0".into(),
        })),
    }
}

#[tokio::test]
async fn bootstrap_with_valid_token() {
    let server = ProvisioningTestServer::start().await;
    server.insert_token("tok-valid-123", "my-agent");

    let mut client = server.client().await;
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tx.send(bootstrap_message("tok-valid-123")).await.unwrap();

    let response = client
        .open_stream(ReceiverStream::new(rx))
        .await
        .unwrap();
    let mut stream = response.into_inner();

    let msg = stream.message().await.unwrap().unwrap();
    match msg.payload.unwrap() {
        ServerPayload::BootstrapResponse(br) => {
            assert_eq!(br.status, BootstrapStatus::BootstrapOk as i32);
            assert_eq!(br.agent_id, "my-agent");
            assert!(!br.secret.is_empty());
            assert!(!br.key_id.is_empty());
            assert!(!br.config_yaml.is_empty());
        }
        other => panic!("expected BootstrapResponse, got {other:?}"),
    }

    assert!(server.agents.get("my-agent").is_some());
}

#[tokio::test]
async fn bootstrap_with_invalid_token() {
    let server = ProvisioningTestServer::start().await;

    let mut client = server.client().await;
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tx.send(bootstrap_message("no-such-token")).await.unwrap();

    let response = client
        .open_stream(ReceiverStream::new(rx))
        .await
        .unwrap();
    let mut stream = response.into_inner();

    let msg = stream.message().await.unwrap().unwrap();
    match msg.payload.unwrap() {
        ServerPayload::BootstrapResponse(br) => {
            assert_eq!(br.status, BootstrapStatus::BootstrapInvalidToken as i32);
        }
        other => panic!("expected BootstrapResponse rejected, got {other:?}"),
    }

    assert_eq!(server.agents.count(), 0);
}

#[tokio::test]
async fn bootstrap_token_consumed_once() {
    let server = ProvisioningTestServer::start().await;
    server.insert_token("tok-once", "agent-once");

    let mut client = server.client().await;

    {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        tx.send(bootstrap_message("tok-once")).await.unwrap();
        let resp = client.open_stream(ReceiverStream::new(rx)).await.unwrap();
        let mut s = resp.into_inner();
        let msg = s.message().await.unwrap().unwrap();
        match msg.payload.unwrap() {
            ServerPayload::BootstrapResponse(br) => {
                assert_eq!(br.status, BootstrapStatus::BootstrapOk as i32);
            }
            other => panic!("expected BootstrapOk, got {other:?}"),
        }
    }

    {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        tx.send(bootstrap_message("tok-once")).await.unwrap();
        let resp = client.open_stream(ReceiverStream::new(rx)).await.unwrap();
        let mut s = resp.into_inner();
        let msg = s.message().await.unwrap().unwrap();
        match msg.payload.unwrap() {
            ServerPayload::BootstrapResponse(br) => {
                assert_eq!(br.status, BootstrapStatus::BootstrapInvalidToken as i32);
            }
            other => panic!("expected rejected on re-use, got {other:?}"),
        }
    }
}
