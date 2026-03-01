use tonic::{Request, Response, Status};

use sentinel_common::proto::push_response::Status as PushStatus;
use sentinel_common::proto::{Batch, PushResponse};
use crate::auth::verify_signature;
use crate::broker::BrokerPublisher;
use crate::store::{AgentStore, IdempotencyStore};

const DEFAULT_GRACE_PERIOD_MS: i64 = 24 * 60 * 60 * 1000;
const DEFAULT_REPLAY_WINDOW_MS: i64 = 5 * 60 * 1000;

pub async fn handle_push_metrics(
    agents: &AgentStore,
    idempotency: &IdempotencyStore,
    broker: &dyn BrokerPublisher,
    request: Request<Batch>,
) -> Result<Response<PushResponse>, Status> {
    handle_push_metrics_with_config(
        agents,
        idempotency,
        broker,
        request,
        DEFAULT_GRACE_PERIOD_MS,
        DEFAULT_REPLAY_WINDOW_MS,
    )
    .await
}

pub async fn handle_push_metrics_with_config(
    agents: &AgentStore,
    idempotency: &IdempotencyStore,
    broker: &dyn BrokerPublisher,
    request: Request<Batch>,
    grace_period_ms: i64,
    replay_window_ms: i64,
) -> Result<Response<PushResponse>, Status> {
    let agent_id = extract_metadata(&request, "x-agent-id")?;
    let signature = extract_metadata(&request, "x-signature")?;
    let key_id = extract_metadata_opt(&request, "x-key-id");

    if agents.get(&agent_id).is_none() {
        return Err(Status::unauthenticated("unknown agent"));
    }

    let secret = agents
        .find_key_secret(&agent_id, key_id.as_deref(), grace_period_ms)
        .ok_or_else(|| Status::unauthenticated("unknown or expired key"))?;

    let batch = request.into_inner();

    let now_ms = current_time_ms();
    if replay_window_ms > 0 && batch.created_at_ms > 0 {
        let age = (now_ms - batch.created_at_ms).abs();
        if age > replay_window_ms {
            return Err(Status::unauthenticated("batch timestamp outside replay window"));
        }
    }

    let canonical = sentinel_common::canonicalize::canonical_bytes(&batch);
    if !verify_signature(&secret, &canonical, &signature) {
        return Err(Status::unauthenticated("invalid signature"));
    }

    if batch.batch_id.is_empty() {
        return Err(Status::invalid_argument("batch_id is required"));
    }

    if idempotency.is_duplicate(&batch.batch_id) {
        return Ok(Response::new(PushResponse {
            status: PushStatus::Ok.into(),
            message: "duplicate, already processed".into(),
        }));
    }

    if let Err(e) = broker
        .publish(&batch, Some(&signature), key_id.as_deref())
        .await
    {
        tracing::error!(batch_id = %batch.batch_id, error = %e, "broker publish failed");
        return Ok(Response::new(PushResponse {
            status: PushStatus::Retry.into(),
            message: "broker unavailable, retry later".into(),
        }));
    }

    idempotency.mark_processed(batch.batch_id.clone(), now_ms);

    Ok(Response::new(PushResponse {
        status: PushStatus::Ok.into(),
        message: "accepted".into(),
    }))
}

fn extract_metadata(request: &Request<Batch>, key: &str) -> Result<String, Status> {
    request
        .metadata()
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| Status::unauthenticated(format!("missing {key} header")))
}

fn extract_metadata_opt(request: &Request<Batch>, key: &str) -> Option<String> {
    request
        .metadata()
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::InMemoryBroker;
    use crate::store::AgentRecord;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use tonic::metadata::MetadataValue;

    type HmacSha256 = Hmac<Sha256>;

    fn setup() -> (AgentStore, IdempotencyStore, InMemoryBroker, Vec<u8>) {
        let agents = AgentStore::new();
        let secret = b"test-secret".to_vec();
        agents.insert(AgentRecord {
            agent_id: "agent-1".into(),
            hw_id: "hw-1".into(),
            secret: secret.clone(),
            key_id: "key-1".into(),
            agent_version: "0.1.0".into(),
            registered_at_ms: 1000,
            deprecated_keys: Vec::new(),
        });
        let idempotency = IdempotencyStore::new();
        let broker = InMemoryBroker::new();
        (agents, idempotency, broker, secret)
    }

    fn sign_batch(secret: &[u8], batch: &Batch) -> String {
        let canonical = sentinel_common::canonicalize::canonical_bytes(batch);
        let mut mac = HmacSha256::new_from_slice(secret).unwrap();
        mac.update(&canonical);
        STANDARD.encode(mac.finalize().into_bytes())
    }

    fn make_request(agent_id: &str, signature: &str, batch: Batch) -> Request<Batch> {
        let mut req = Request::new(batch);
        req.metadata_mut().insert(
            "x-agent-id",
            MetadataValue::try_from(agent_id).unwrap(),
        );
        req.metadata_mut().insert(
            "x-signature",
            MetadataValue::try_from(signature).unwrap(),
        );
        req
    }

    fn sample_batch() -> Batch {
        Batch {
            agent_id: "agent-1".into(),
            batch_id: "b-1".into(),
            seq_start: 0,
            seq_end: 1,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn valid_push_accepted() {
        let (agents, idem, broker, secret) = setup();
        let batch = sample_batch();
        let sig = sign_batch(&secret, &batch);
        let req = make_request("agent-1", &sig, batch);
        let resp = handle_push_metrics(&agents, &idem, &broker, req)
            .await
            .unwrap();
        assert_eq!(resp.into_inner().status, PushStatus::Ok as i32);
        assert_eq!(broker.published_count(), 1);
    }

    #[tokio::test]
    async fn duplicate_batch_deduped() {
        let (agents, idem, broker, secret) = setup();
        let batch = sample_batch();
        let sig = sign_batch(&secret, &batch);

        let req1 = make_request("agent-1", &sig, batch.clone());
        handle_push_metrics(&agents, &idem, &broker, req1)
            .await
            .unwrap();

        let req2 = make_request("agent-1", &sig, batch);
        let resp = handle_push_metrics(&agents, &idem, &broker, req2)
            .await
            .unwrap();
        assert_eq!(resp.into_inner().status, PushStatus::Ok as i32);
        assert_eq!(broker.published_count(), 1);
    }

    #[tokio::test]
    async fn unknown_agent_rejected() {
        let (agents, idem, broker, _) = setup();
        let batch = sample_batch();
        let req = make_request("agent-unknown", "sig", batch);
        let err = handle_push_metrics(&agents, &idem, &broker, req)
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[tokio::test]
    async fn bad_signature_rejected() {
        let (agents, idem, broker, _) = setup();
        let batch = sample_batch();
        let req = make_request("agent-1", "wrong-sig", batch);
        let err = handle_push_metrics(&agents, &idem, &broker, req)
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[tokio::test]
    async fn empty_batch_id_rejected() {
        let (agents, idem, broker, secret) = setup();
        let mut batch = sample_batch();
        batch.batch_id = "".into();
        let sig = sign_batch(&secret, &batch);
        let req = make_request("agent-1", &sig, batch);
        let err = handle_push_metrics(&agents, &idem, &broker, req)
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }
}
