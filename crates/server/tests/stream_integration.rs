use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use sentinel_common::crypto::sign_data;
use sentinel_common::proto::agent_message::Payload as AgentPayload;
use sentinel_common::proto::sentinel_stream_client::SentinelStreamClient;
use sentinel_common::proto::sentinel_stream_server::SentinelStreamServer;
use sentinel_common::proto::server_message::Payload as ServerPayload;
use sentinel_common::proto::{
    AgentMessage, BatchAckStatus, HandshakeRequest, HandshakeStatus, Metric, MetricsBatch,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Channel, Server};

use sentinel_server::broker::InMemoryBroker;
use sentinel_server::store::{AgentRecord, AgentStore, IdempotencyStore};
use sentinel_server::stream::{PresenceEventBus, SessionRegistry, StreamService};

struct StreamTestServer {
    addr: std::net::SocketAddr,
    agents: AgentStore,
    broker: InMemoryBroker,
    registry: SessionRegistry,
    events: PresenceEventBus,
    shutdown: Arc<AtomicBool>,
}

impl StreamTestServer {
    async fn start() -> Self {
        let agents = AgentStore::new();
        let idempotency = IdempotencyStore::new();
        let broker = InMemoryBroker::new();
        let registry = SessionRegistry::new();
        let events = PresenceEventBus::new();
        let shutdown = Arc::new(AtomicBool::new(false));

        let svc = StreamService::new(
            agents.clone(),
            idempotency,
            Arc::new(broker.clone()),
            registry.clone(),
            events.clone(),
            300_000,
        );

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
            broker,
            registry,
            events,
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

    fn insert_agent(&self, agent_id: &str, secret: &[u8]) -> AgentRecord {
        let record = AgentRecord {
            agent_id: agent_id.into(),
            hw_id: format!("hw-{agent_id}"),
            secret: secret.to_vec(),
            key_id: format!("key-{agent_id}"),
            agent_version: "1.0.0".into(),
            registered_at_ms: now_ms(),
            deprecated_keys: Vec::new(),
            last_seen: None,
        };
        self.agents.insert(record.clone());
        record
    }
}

impl Drop for StreamTestServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn build_handshake(agent_id: &str, key_id: &str, secret: &[u8]) -> AgentMessage {
    let ts = now_ms();
    let canonical = format!("{agent_id}:{ts}:{key_id}");
    let signature = sign_data(secret, canonical.as_bytes());
    AgentMessage {
        payload: Some(AgentPayload::Handshake(HandshakeRequest {
            agent_id: agent_id.into(),
            agent_version: "1.0.0".into(),
            capabilities: vec!["metrics".into()],
            key_id: key_id.into(),
            timestamp_ms: ts,
            signature,
        })),
    }
}

fn build_metrics_batch(batch_id: &str) -> AgentMessage {
    AgentMessage {
        payload: Some(AgentPayload::MetricsBatch(MetricsBatch {
            batch_id: batch_id.into(),
            seq_start: 1,
            seq_end: 1,
            created_at_ms: now_ms(),
            metrics: vec![Metric {
                name: "cpu.usage".into(),
                labels: Default::default(),
                rtype: 1,
                value: Some(sentinel_common::proto::metric::Value::ValueDouble(42.0)),
                timestamp_ms: now_ms(),
            }],
            meta: Default::default(),
            signature: String::new(),
        })),
    }
}

#[tokio::test]
async fn handshake_then_batch_ack() {
    let server = StreamTestServer::start().await;
    let secret = b"test-secret-1234";
    let record = server.insert_agent("agent-1", secret);

    let mut client = server.client().await;
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tx.send(build_handshake("agent-1", &record.key_id, secret))
        .await
        .unwrap();

    let response = client.open_stream(ReceiverStream::new(rx)).await.unwrap();
    let mut stream = response.into_inner();

    let ack = stream.message().await.unwrap().unwrap();
    let payload = ack.payload.unwrap();
    match payload {
        ServerPayload::HandshakeAck(h) => {
            assert_eq!(h.status, HandshakeStatus::HandshakeOk as i32);
            assert!(h.heartbeat_interval_ms > 0);
        }
        other => panic!("expected HandshakeAck, got {other:?}"),
    }

    tx.send(build_metrics_batch("batch-001")).await.unwrap();

    let batch_ack = stream.message().await.unwrap().unwrap();
    match batch_ack.payload.unwrap() {
        ServerPayload::BatchAck(ba) => {
            assert_eq!(ba.batch_id, "batch-001");
            assert_eq!(ba.status, BatchAckStatus::BatchAccepted as i32);
        }
        other => panic!("expected BatchAck, got {other:?}"),
    }

    assert_eq!(server.broker.published_count(), 1);
    assert!(server.registry.contains("agent-1"));
}

#[tokio::test]
async fn handshake_rejected_unknown_agent() {
    let server = StreamTestServer::start().await;
    let mut client = server.client().await;
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tx.send(build_handshake("no-such-agent", "key-fake", b"fake"))
        .await
        .unwrap();

    let response = client.open_stream(ReceiverStream::new(rx)).await.unwrap();
    let mut stream = response.into_inner();

    let msg = stream.message().await.unwrap().unwrap();
    match msg.payload.unwrap() {
        ServerPayload::HandshakeAck(h) => {
            assert_eq!(h.status, HandshakeStatus::HandshakeRejected as i32);
        }
        other => panic!("expected HandshakeAck rejected, got {other:?}"),
    }

    assert!(!server.registry.contains("no-such-agent"));
}

#[tokio::test]
async fn duplicate_batch_idempotent() {
    let server = StreamTestServer::start().await;
    let secret = b"dup-secret-0000";
    let record = server.insert_agent("agent-dup", secret);

    let mut client = server.client().await;
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tx.send(build_handshake("agent-dup", &record.key_id, secret))
        .await
        .unwrap();

    let response = client.open_stream(ReceiverStream::new(rx)).await.unwrap();
    let mut stream = response.into_inner();

    let _handshake_ack = stream.message().await.unwrap().unwrap();

    tx.send(build_metrics_batch("batch-dup")).await.unwrap();
    let first = stream.message().await.unwrap().unwrap();
    match first.payload.unwrap() {
        ServerPayload::BatchAck(ba) => {
            assert_eq!(ba.status, BatchAckStatus::BatchAccepted as i32);
        }
        other => panic!("expected BatchAck, got {other:?}"),
    }

    tx.send(build_metrics_batch("batch-dup")).await.unwrap();
    let second = stream.message().await.unwrap().unwrap();
    match second.payload.unwrap() {
        ServerPayload::BatchAck(ba) => {
            assert_eq!(ba.batch_id, "batch-dup");
            assert_eq!(ba.status, BatchAckStatus::BatchAccepted as i32);
            assert!(ba.message.contains("duplicate"));
        }
        other => panic!("expected duplicate BatchAck, got {other:?}"),
    }

    assert_eq!(server.broker.published_count(), 1);
}

#[tokio::test]
async fn presence_events_on_connect_disconnect() {
    let server = StreamTestServer::start().await;
    let secret = b"presence-secret!";
    let record = server.insert_agent("agent-presence", secret);

    let mut rx_events = server.events.subscribe();

    let mut client = server.client().await;
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tx.send(build_handshake("agent-presence", &record.key_id, secret))
        .await
        .unwrap();

    let response = client.open_stream(ReceiverStream::new(rx)).await.unwrap();
    let mut stream = response.into_inner();
    let _ack = stream.message().await.unwrap().unwrap();

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx_events.recv())
        .await
        .unwrap()
        .unwrap();

    match &event {
        sentinel_server::stream::PresenceEvent::AgentConnected { agent_id, .. } => {
            assert_eq!(agent_id, "agent-presence");
        }
        other => panic!("expected AgentConnected, got {other:?}"),
    }

    drop(tx);
    drop(stream);

    let disconnect = tokio::time::timeout(std::time::Duration::from_secs(2), rx_events.recv())
        .await
        .unwrap()
        .unwrap();

    match &disconnect {
        sentinel_server::stream::PresenceEvent::AgentDisconnected { agent_id, .. } => {
            assert_eq!(agent_id, "agent-presence");
        }
        other => panic!("expected AgentDisconnected, got {other:?}"),
    }
}
