use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use sentinel_common::crypto::sign_data;
use sentinel_common::proto::agent_message::Payload as AgentPayload;
use sentinel_common::proto::server_message::Payload as ServerPayload;
use sentinel_common::proto::sentinel_stream_client::SentinelStreamClient;
use sentinel_common::proto::sentinel_stream_server::SentinelStreamServer;
use sentinel_common::proto::{
    AgentMessage, HandshakeRequest, HandshakeStatus, HeartbeatPing, SystemStats,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Channel, Server};

use sentinel_server::broker::InMemoryBroker;
use sentinel_server::store::{AgentRecord, AgentStore, IdempotencyStore};
use sentinel_server::stream::{PresenceEventBus, SessionRegistry, StreamService};

struct HeartbeatTestServer {
    addr: std::net::SocketAddr,
    agents: AgentStore,
    registry: SessionRegistry,
    events: PresenceEventBus,
    shutdown: Arc<AtomicBool>,
}

impl HeartbeatTestServer {
    async fn start() -> Self {
        let agents = AgentStore::new();
        let idempotency = IdempotencyStore::new();
        let broker = Arc::new(InMemoryBroker::new());
        let registry = SessionRegistry::new();
        let events = PresenceEventBus::new();
        let shutdown = Arc::new(AtomicBool::new(false));

        let svc = StreamService::new(
            agents.clone(),
            idempotency,
            broker,
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

impl Drop for HeartbeatTestServer {
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

fn build_heartbeat_ping() -> AgentMessage {
    AgentMessage {
        payload: Some(AgentPayload::HeartbeatPing(HeartbeatPing {
            timestamp_ms: now_ms(),
            system_stats: Some(SystemStats {
                cpu_percent: 45.5,
                memory_used_bytes: 4_000_000_000,
                memory_total_bytes: 8_000_000_000,
                disk_used_bytes: 100_000_000_000,
                disk_total_bytes: 500_000_000_000,
                load_avg_1m: 1.2,
                process_count: 150,
                uptime_seconds: 86400,
                os_name: "Linux".into(),
                hostname: "test-host".into(),
            }),
        })),
    }
}

#[tokio::test]
async fn heartbeat_ping_pong() {
    let server = HeartbeatTestServer::start().await;
    let secret = b"hb-secret-12345";
    let record = server.insert_agent("agent-hb", secret);

    let mut client = server.client().await;
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tx.send(build_handshake("agent-hb", &record.key_id, secret))
        .await
        .unwrap();

    let response = client
        .open_stream(ReceiverStream::new(rx))
        .await
        .unwrap();
    let mut stream = response.into_inner();

    let ack = stream.message().await.unwrap().unwrap();
    match ack.payload.unwrap() {
        ServerPayload::HandshakeAck(h) => {
            assert_eq!(h.status, HandshakeStatus::HandshakeOk as i32);
        }
        other => panic!("expected HandshakeAck, got {other:?}"),
    }

    tx.send(build_heartbeat_ping()).await.unwrap();

    let pong = stream.message().await.unwrap().unwrap();
    match pong.payload.unwrap() {
        ServerPayload::HeartbeatPong(p) => {
            assert!(p.server_time_ms > 0);
        }
        other => panic!("expected HeartbeatPong, got {other:?}"),
    }

    let snap = server.registry.snapshot("agent-hb").unwrap();
    assert!(snap.heartbeat_count >= 1);
}

#[tokio::test]
async fn heartbeat_emits_presence_event() {
    let server = HeartbeatTestServer::start().await;
    let secret = b"hb-presence-sec";
    let record = server.insert_agent("agent-hb-ev", secret);

    let mut rx_events = server.events.subscribe();

    let mut client = server.client().await;
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tx.send(build_handshake("agent-hb-ev", &record.key_id, secret))
        .await
        .unwrap();

    let response = client
        .open_stream(ReceiverStream::new(rx))
        .await
        .unwrap();
    let mut stream = response.into_inner();
    let _ack = stream.message().await.unwrap().unwrap();

    let _connected = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        rx_events.recv(),
    )
    .await
    .unwrap()
    .unwrap();

    tx.send(build_heartbeat_ping()).await.unwrap();
    let _pong = stream.message().await.unwrap().unwrap();

    let evt = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        rx_events.recv(),
    )
    .await
    .unwrap()
    .unwrap();

    match &evt {
        sentinel_server::stream::PresenceEvent::HeartbeatReceived {
            agent_id,
            cpu_percent,
            ..
        } => {
            assert_eq!(agent_id, "agent-hb-ev");
            assert!(*cpu_percent > 40.0);
        }
        other => panic!("expected HeartbeatReceived, got {other:?}"),
    }
}

#[tokio::test]
async fn session_registered_after_handshake() {
    let server = HeartbeatTestServer::start().await;
    let secret = b"session-secret!!";
    let record = server.insert_agent("agent-sess", secret);

    assert!(!server.registry.contains("agent-sess"));

    let mut client = server.client().await;
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tx.send(build_handshake("agent-sess", &record.key_id, secret))
        .await
        .unwrap();

    let response = client
        .open_stream(ReceiverStream::new(rx))
        .await
        .unwrap();
    let mut stream = response.into_inner();
    let _ack = stream.message().await.unwrap().unwrap();

    assert!(server.registry.contains("agent-sess"));
    assert_eq!(server.registry.connected_count(), 1);

    let snap = server.registry.snapshot("agent-sess").unwrap();
    assert_eq!(snap.agent_version, "1.0.0");
}

#[tokio::test]
async fn stale_detection_after_timeout() {
    let server = HeartbeatTestServer::start().await;
    let secret = b"stale-secret1234";
    let record = server.insert_agent("agent-stale", secret);

    let mut client = server.client().await;
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tx.send(build_handshake("agent-stale", &record.key_id, secret))
        .await
        .unwrap();

    let response = client
        .open_stream(ReceiverStream::new(rx))
        .await
        .unwrap();
    let mut stream = response.into_inner();
    let _ack = stream.message().await.unwrap().unwrap();

    assert!(server.registry.find_stale(1_000_000).is_empty());

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let stale = server.registry.find_stale(10);
    assert!(!stale.is_empty());
    assert_eq!(stale[0].0, "agent-stale");
}
