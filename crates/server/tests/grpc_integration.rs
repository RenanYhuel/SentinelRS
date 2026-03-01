use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use sentinel_common::proto::agent_service_server::AgentServiceServer;
use sentinel_common::proto::metric::Value;
use sentinel_common::proto::{Batch, Metric, RegisterRequest};
use sentinel_common::proto::agent_service_client::AgentServiceClient;
use tonic::transport::{Channel, Server};
use tonic::Request;

use sentinel_server::broker::InMemoryBroker;
use sentinel_server::grpc::AgentServiceImpl;
use sentinel_server::store::{AgentStore, IdempotencyStore};

fn sample_metrics(n: usize) -> Vec<Metric> {
    (0..n)
        .map(|i| Metric {
            name: format!("cpu.core.{i}.usage"),
            labels: Default::default(),
            rtype: 1,
            value: Some(Value::ValueDouble(i as f64 * 10.0)),
            timestamp_ms: 1700000000000 + i as i64,
        })
        .collect()
}

struct TestServer {
    addr: std::net::SocketAddr,
    agents: AgentStore,
    broker: InMemoryBroker,
    shutdown: Arc<AtomicBool>,
}

impl TestServer {
    async fn start() -> Self {
        let agents = AgentStore::new();
        let idempotency = IdempotencyStore::new();
        let broker = InMemoryBroker::new();
        let shutdown = Arc::new(AtomicBool::new(false));

        let svc = AgentServiceImpl::new(agents.clone(), idempotency, broker.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let shutdown_flag = shutdown.clone();
        tokio::spawn(async move {
            let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
            Server::builder()
                .add_service(AgentServiceServer::new(svc))
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
            shutdown,
        }
    }

    async fn client(&self) -> AgentServiceClient<Channel> {
        let endpoint = format!("http://{}", self.addr);
        let channel = Channel::from_shared(endpoint)
            .unwrap()
            .connect()
            .await
            .unwrap();
        AgentServiceClient::new(channel)
    }

    fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[tokio::test]
async fn register_new_agent() {
    let server = TestServer::start().await;
    let mut client = server.client().await;

    let resp = client
        .register(Request::new(RegisterRequest {
            hw_id: "hw-test-001".into(),
            agent_version: "0.1.0".into(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert!(!resp.agent_id.is_empty());
    assert!(!resp.secret.is_empty());

    let stored = server.agents.get(&resp.agent_id);
    assert!(stored.is_some());
}

#[tokio::test]
async fn register_same_hw_returns_existing() {
    let server = TestServer::start().await;
    let mut client = server.client().await;

    let r1 = client
        .register(Request::new(RegisterRequest {
            hw_id: "hw-dup".into(),
            agent_version: "0.1.0".into(),
        }))
        .await
        .unwrap()
        .into_inner();

    let r2 = client
        .register(Request::new(RegisterRequest {
            hw_id: "hw-dup".into(),
            agent_version: "0.1.0".into(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(r1.agent_id, r2.agent_id);
}

#[tokio::test]
async fn push_metrics_publishes_to_broker() {
    let server = TestServer::start().await;
    let mut client = server.client().await;

    let reg = client
        .register(Request::new(RegisterRequest {
            hw_id: "hw-push-test".into(),
            agent_version: "0.1.0".into(),
        }))
        .await
        .unwrap()
        .into_inner();

    let agent = server.agents.get(&reg.agent_id).unwrap();

    let batch = Batch {
        agent_id: reg.agent_id.clone(),
        batch_id: "batch-grpc-001".into(),
        seq_start: 0,
        seq_end: 3,
        created_at_ms: 1_700_000_000_000,
        metrics: sample_metrics(3),
        meta: Default::default(),
    };

    let canonical = sentinel_common::canonicalize::canonical_bytes(&batch);
    let signature = sentinel_common::crypto::sign_data(&agent.secret, &canonical);

    let mut request = Request::new(batch);
    request
        .metadata_mut()
        .insert("x-agent-id", reg.agent_id.parse().unwrap());
    request
        .metadata_mut()
        .insert("x-signature", signature.parse().unwrap());
    request
        .metadata_mut()
        .insert("x-key-id", "default".parse().unwrap());

    let resp = client.push_metrics(request).await.unwrap().into_inner();
    assert_eq!(resp.status, 0);
    assert_eq!(server.broker.published_count(), 1);
}

#[tokio::test]
async fn push_duplicate_batch_deduped() {
    let server = TestServer::start().await;
    let mut client = server.client().await;

    let reg = client
        .register(Request::new(RegisterRequest {
            hw_id: "hw-dedup".into(),
            agent_version: "0.1.0".into(),
        }))
        .await
        .unwrap()
        .into_inner();

    let agent = server.agents.get(&reg.agent_id).unwrap();

    let batch = Batch {
        agent_id: reg.agent_id.clone(),
        batch_id: "batch-dup-001".into(),
        seq_start: 0,
        seq_end: 1,
        created_at_ms: 1_700_000_000_000,
        metrics: sample_metrics(1),
        meta: Default::default(),
    };

    let canonical = sentinel_common::canonicalize::canonical_bytes(&batch);
    let signature = sentinel_common::crypto::sign_data(&agent.secret, &canonical);

    for _ in 0..3 {
        let mut request = Request::new(batch.clone());
        request
            .metadata_mut()
            .insert("x-agent-id", reg.agent_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-signature", signature.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-key-id", "default".parse().unwrap());

        let resp = client.push_metrics(request).await.unwrap().into_inner();
        assert_eq!(resp.status, 0);
    }

    assert_eq!(server.broker.published_count(), 1);
}

#[tokio::test]
async fn heartbeat_for_registered_agent() {
    let server = TestServer::start().await;
    let mut client = server.client().await;

    let reg = client
        .register(Request::new(RegisterRequest {
            hw_id: "hw-hb".into(),
            agent_version: "0.1.0".into(),
        }))
        .await
        .unwrap()
        .into_inner();

    let resp = client
        .send_heartbeat(Request::new(sentinel_common::proto::Heartbeat {
            agent_id: reg.agent_id,
            ts_ms: 1_700_000_000_000,
            info: Default::default(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(resp.status, 0);
}

#[tokio::test]
async fn push_metrics_unknown_agent_rejected() {
    let server = TestServer::start().await;
    let mut client = server.client().await;

    let batch = Batch {
        agent_id: "nonexistent-agent".into(),
        batch_id: "batch-unknown".into(),
        seq_start: 0,
        seq_end: 1,
        created_at_ms: 1_700_000_000_000,
        metrics: sample_metrics(1),
        meta: Default::default(),
    };

    let mut request = Request::new(batch);
    request
        .metadata_mut()
        .insert("x-agent-id", "nonexistent-agent".parse().unwrap());
    request
        .metadata_mut()
        .insert("x-signature", "invalid".parse().unwrap());

    let result = client.push_metrics(request).await;
    assert!(result.is_err());
}
