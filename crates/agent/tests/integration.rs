use std::sync::{Arc, Mutex};

use sentinel_common::proto::agent_service_server::{AgentService, AgentServiceServer};
use sentinel_common::proto::metric::Value;
use sentinel_common::proto::{
    Batch, Heartbeat, Metric, PushResponse, RegisterRequest, RegisterResponse,
};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

struct StubService {
    received: Arc<Mutex<Vec<Batch>>>,
}

#[tonic::async_trait]
impl AgentService for StubService {
    async fn register(
        &self,
        _request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        Ok(Response::new(RegisterResponse {
            agent_id: "test-agent".into(),
            secret: "test-secret".into(),
        }))
    }

    async fn push_metrics(
        &self,
        request: Request<Batch>,
    ) -> Result<Response<PushResponse>, Status> {
        let batch = request.into_inner();
        self.received.lock().unwrap().push(batch);
        Ok(Response::new(PushResponse {
            status: 0,
            message: "ok".into(),
        }))
    }

    async fn send_heartbeat(
        &self,
        _request: Request<Heartbeat>,
    ) -> Result<Response<PushResponse>, Status> {
        Ok(Response::new(PushResponse {
            status: 0,
            message: "ok".into(),
        }))
    }
}

fn sample_metrics(n: usize) -> Vec<Metric> {
    (0..n)
        .map(|i| Metric {
            name: format!("cpu.usage.{i}"),
            labels: Default::default(),
            rtype: 1,
            value: Some(Value::ValueDouble(i as f64)),
            timestamp_ms: 1000 + i as i64,
        })
        .collect()
}

#[tokio::test]
async fn agent_sends_batch_and_server_receives_valid_data() {
    let received = Arc::new(Mutex::new(Vec::<Batch>::new()));
    let svc = StubService {
        received: received.clone(),
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = tokio::spawn(async move {
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        Server::builder()
            .add_service(AgentServiceServer::new(svc))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let endpoint = format!("http://{addr}");

    use sentinel_common::proto::agent_service_client::AgentServiceClient;
    use tonic::transport::Channel;

    let channel = Channel::from_shared(endpoint)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = AgentServiceClient::new(channel);

    let batch = Batch {
        agent_id: "test-agent".into(),
        batch_id: "batch-001".into(),
        seq_start: 0,
        seq_end: 3,
        created_at_ms: 1700000000000,
        metrics: sample_metrics(3),
        meta: Default::default(),
    };

    let resp = client
        .push_metrics(Request::new(batch.clone()))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp.status, 0);

    let stored = received.lock().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].agent_id, "test-agent");
    assert_eq!(stored[0].batch_id, "batch-001");
    assert_eq!(stored[0].metrics.len(), 3);
    assert_eq!(stored[0].seq_start, 0);
    assert_eq!(stored[0].seq_end, 3);

    server_handle.abort();
}

#[tokio::test]
async fn wal_to_send_to_ack_compaction_flow() {
    let received = Arc::new(Mutex::new(Vec::<Batch>::new()));
    let svc = StubService {
        received: received.clone(),
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = tokio::spawn(async move {
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        Server::builder()
            .add_service(AgentServiceServer::new(svc))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let wal_dir = tempfile::tempdir().unwrap();

    use sentinel_agent::buffer::Wal;
    use sentinel_agent::batch::BatchComposer;

    let mut wal = Wal::open(wal_dir.path(), false, 1024 * 1024).unwrap();
    let mut composer = BatchComposer::new("agent-wal-test".into(), 0);

    let batch = composer.compose(sample_metrics(5));
    let encoded = BatchComposer::encode_batch(&batch);
    wal.append(encoded).unwrap();

    let unacked: Vec<_> = wal.iter_unacked().unwrap();
    assert_eq!(unacked.len(), 1);

    let endpoint = format!("http://{addr}");
    use sentinel_common::proto::agent_service_client::AgentServiceClient;
    use tonic::transport::Channel;

    let channel = Channel::from_shared(endpoint)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = AgentServiceClient::new(channel);

    for (record_id, data) in wal.iter_unacked().unwrap() {
        let decoded = BatchComposer::decode_batch(&data).unwrap();
        let resp = client
            .push_metrics(Request::new(decoded))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(resp.status, 0);
        wal.ack(record_id);
    }
    wal.save_meta().unwrap();

    let remaining: Vec<_> = wal.iter_unacked().unwrap();
    assert!(remaining.is_empty());

    let stored = received.lock().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].agent_id, "agent-wal-test");
    assert_eq!(stored[0].metrics.len(), 5);

    server_handle.abort();
}
