use sentinel_common::canonicalize::canonical_bytes;
use sentinel_common::crypto::{generate_secret, sign_data};
use sentinel_common::proto::push_response::Status as PushStatus;
use sentinel_common::proto::Batch;
use sentinel_server::broker::{BrokerPublisher, InMemoryBroker};
use sentinel_server::grpc::push_metrics::handle_push_metrics_with_config;
use sentinel_server::store::{AgentRecord, AgentStore, DeprecatedKey, IdempotencyStore};
use tonic::metadata::MetadataValue;
use tonic::Request;

const GRACE_PERIOD_MS: i64 = 60_000;
const REPLAY_WINDOW_MS: i64 = 5_000;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn make_batch(batch_id: &str) -> Batch {
    Batch {
        agent_id: "agent-sec".into(),
        batch_id: batch_id.into(),
        seq_start: 0,
        seq_end: 1,
        created_at_ms: now_ms(),
        ..Default::default()
    }
}

fn build_request(agent_id: &str, sig: &str, key_id: Option<&str>, batch: Batch) -> Request<Batch> {
    let mut req = Request::new(batch);
    req.metadata_mut()
        .insert("x-agent-id", MetadataValue::try_from(agent_id).unwrap());
    req.metadata_mut()
        .insert("x-signature", MetadataValue::try_from(sig).unwrap());
    if let Some(kid) = key_id {
        req.metadata_mut()
            .insert("x-key-id", MetadataValue::try_from(kid).unwrap());
    }
    req
}

fn seed(secret: &[u8], key_id: &str) -> (AgentStore, IdempotencyStore, InMemoryBroker) {
    let agents = AgentStore::new();
    agents.insert(AgentRecord {
        agent_id: "agent-sec".into(),
        hw_id: "hw-sec".into(),
        secret: secret.to_vec(),
        key_id: key_id.into(),
        agent_version: "0.1.0".into(),
        registered_at_ms: 1000,
        deprecated_keys: Vec::new(),
    });
    (agents, IdempotencyStore::new(), InMemoryBroker::new())
}

fn broker_ref(b: &InMemoryBroker) -> &dyn BrokerPublisher {
    b as &dyn BrokerPublisher
}

#[tokio::test]
async fn tampered_signature_rejected() {
    let secret = generate_secret();
    let (agents, idem, broker) = seed(&secret, "k-1");

    let batch = make_batch("tamper-1");
    let real_sig = sign_data(&secret, &canonical_bytes(&batch));

    let tampered = format!("{}X", &real_sig[..real_sig.len() - 1]);
    let req = build_request("agent-sec", &tampered, Some("k-1"), batch);

    let err = handle_push_metrics_with_config(
        &agents,
        &idem,
        broker_ref(&broker),
        req,
        GRACE_PERIOD_MS,
        REPLAY_WINDOW_MS,
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
    assert!(err.message().contains("invalid signature"));
}

#[tokio::test]
async fn tampered_payload_rejected() {
    let secret = generate_secret();
    let (agents, idem, broker) = seed(&secret, "k-1");

    let batch = make_batch("tamper-2");
    let sig = sign_data(&secret, &canonical_bytes(&batch));

    let mut altered = batch.clone();
    altered.seq_end = 999;

    let req = build_request("agent-sec", &sig, Some("k-1"), altered);
    let err = handle_push_metrics_with_config(
        &agents,
        &idem,
        broker_ref(&broker),
        req,
        GRACE_PERIOD_MS,
        REPLAY_WINDOW_MS,
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn replay_old_batch_rejected() {
    let secret = generate_secret();
    let (agents, idem, broker) = seed(&secret, "k-1");

    let mut batch = make_batch("replay-1");
    batch.created_at_ms = now_ms() - 60_000;
    let sig = sign_data(&secret, &canonical_bytes(&batch));

    let req = build_request("agent-sec", &sig, Some("k-1"), batch);
    let err = handle_push_metrics_with_config(
        &agents,
        &idem,
        broker_ref(&broker),
        req,
        GRACE_PERIOD_MS,
        REPLAY_WINDOW_MS,
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
    assert!(err.message().contains("replay window"));
}

#[tokio::test]
async fn future_batch_rejected() {
    let secret = generate_secret();
    let (agents, idem, broker) = seed(&secret, "k-1");

    let mut batch = make_batch("replay-2");
    batch.created_at_ms = now_ms() + 60_000;
    let sig = sign_data(&secret, &canonical_bytes(&batch));

    let req = build_request("agent-sec", &sig, Some("k-1"), batch);
    let err = handle_push_metrics_with_config(
        &agents,
        &idem,
        broker_ref(&broker),
        req,
        GRACE_PERIOD_MS,
        REPLAY_WINDOW_MS,
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
    assert!(err.message().contains("replay window"));
}

#[tokio::test]
async fn rotated_key_within_grace_accepted() {
    let old_secret = generate_secret();
    let new_secret = generate_secret();
    let (agents, idem, broker) = seed(&new_secret, "k-2");

    {
        let mut entry = agents.get("agent-sec").unwrap();
        entry.deprecated_keys.push(DeprecatedKey {
            key_id: "k-1".into(),
            secret: old_secret.clone(),
            deprecated_at_ms: now_ms(),
        });
        agents.insert(entry.clone());
    }

    let batch = make_batch("rotate-ok");
    let sig = sign_data(&old_secret, &canonical_bytes(&batch));

    let req = build_request("agent-sec", &sig, Some("k-1"), batch);
    let resp = handle_push_metrics_with_config(
        &agents,
        &idem,
        broker_ref(&broker),
        req,
        GRACE_PERIOD_MS,
        REPLAY_WINDOW_MS,
    )
    .await
    .unwrap();
    assert_eq!(resp.into_inner().status, PushStatus::Ok as i32);
}

#[tokio::test]
async fn rotated_key_expired_rejected() {
    let old_secret = generate_secret();
    let new_secret = generate_secret();
    let (agents, idem, broker) = seed(&new_secret, "k-2");

    {
        let mut entry = agents.get("agent-sec").unwrap();
        entry.deprecated_keys.push(DeprecatedKey {
            key_id: "k-1".into(),
            secret: old_secret.clone(),
            deprecated_at_ms: now_ms() - GRACE_PERIOD_MS - 1000,
        });
        agents.insert(entry.clone());
    }

    let batch = make_batch("rotate-expired");
    let sig = sign_data(&old_secret, &canonical_bytes(&batch));

    let req = build_request("agent-sec", &sig, Some("k-1"), batch);
    let err = handle_push_metrics_with_config(
        &agents,
        &idem,
        broker_ref(&broker),
        req,
        GRACE_PERIOD_MS,
        REPLAY_WINDOW_MS,
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
    assert!(err.message().contains("unknown or expired key"));
}

#[tokio::test]
async fn current_key_accepted() {
    let secret = generate_secret();
    let (agents, idem, broker) = seed(&secret, "k-1");

    let batch = make_batch("current-key");
    let sig = sign_data(&secret, &canonical_bytes(&batch));

    let req = build_request("agent-sec", &sig, Some("k-1"), batch);
    let resp = handle_push_metrics_with_config(
        &agents,
        &idem,
        broker_ref(&broker),
        req,
        GRACE_PERIOD_MS,
        REPLAY_WINDOW_MS,
    )
    .await
    .unwrap();
    assert_eq!(resp.into_inner().status, PushStatus::Ok as i32);
    assert_eq!(broker.published_count(), 1);
}

#[tokio::test]
async fn no_key_id_uses_current_key() {
    let secret = generate_secret();
    let (agents, idem, broker) = seed(&secret, "k-1");

    let batch = make_batch("no-kid");
    let sig = sign_data(&secret, &canonical_bytes(&batch));

    let req = build_request("agent-sec", &sig, None, batch);
    let resp = handle_push_metrics_with_config(
        &agents,
        &idem,
        broker_ref(&broker),
        req,
        GRACE_PERIOD_MS,
        REPLAY_WINDOW_MS,
    )
    .await
    .unwrap();
    assert_eq!(resp.into_inner().status, PushStatus::Ok as i32);
}
