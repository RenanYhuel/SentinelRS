use sentinel_common::proto::{
    server_message::Payload, Batch, BatchAck, BatchAckStatus, MetricsBatch, ServerMessage,
};

use crate::auth::verify_signature;
use crate::broker::BrokerPublisher;
use crate::store::{AgentStore, IdempotencyStore};

pub async fn handle_metrics_batch(
    agent_id: &str,
    key_id: &str,
    batch: MetricsBatch,
    agents: &AgentStore,
    idempotency: &IdempotencyStore,
    broker: &dyn BrokerPublisher,
    grace_period_ms: i64,
) -> ServerMessage {
    if batch.batch_id.is_empty() {
        return ack_message(
            &batch.batch_id,
            BatchAckStatus::BatchRejected,
            "batch_id is required",
        );
    }

    if idempotency.is_duplicate(&batch.batch_id) {
        return ack_message(
            &batch.batch_id,
            BatchAckStatus::BatchAccepted,
            "duplicate, already processed",
        );
    }

    let secret = match agents.find_key_secret(agent_id, Some(key_id), grace_period_ms) {
        Some(s) => s,
        None => {
            return ack_message(
                &batch.batch_id,
                BatchAckStatus::BatchRejected,
                "unknown or expired key",
            );
        }
    };

    let legacy_batch = to_legacy_batch(agent_id, &batch);
    let canonical = sentinel_common::canonicalize::canonical_bytes(&legacy_batch);

    if !batch.signature.is_empty() && !verify_signature(&secret, &canonical, &batch.signature) {
        return ack_message(
            &batch.batch_id,
            BatchAckStatus::BatchRejected,
            "invalid batch signature",
        );
    }

    let signature_str = if batch.signature.is_empty() {
        None
    } else {
        Some(batch.signature.as_str())
    };

    if let Err(e) = broker
        .publish(&legacy_batch, signature_str, Some(key_id))
        .await
    {
        tracing::error!(batch_id = %batch.batch_id, error = %e, "broker publish failed");
        return ack_message(
            &batch.batch_id,
            BatchAckStatus::BatchRetry,
            "broker unavailable",
        );
    }

    let now_ms = current_time_ms();
    idempotency.mark_processed(batch.batch_id.clone(), now_ms);

    ack_message(&batch.batch_id, BatchAckStatus::BatchAccepted, "accepted")
}

fn to_legacy_batch(agent_id: &str, mb: &MetricsBatch) -> Batch {
    Batch {
        agent_id: agent_id.into(),
        batch_id: mb.batch_id.clone(),
        seq_start: mb.seq_start,
        seq_end: mb.seq_end,
        created_at_ms: mb.created_at_ms,
        metrics: mb.metrics.clone(),
        meta: mb.meta.clone(),
    }
}

fn ack_message(batch_id: &str, status: BatchAckStatus, message: &str) -> ServerMessage {
    ServerMessage {
        payload: Some(Payload::BatchAck(BatchAck {
            batch_id: batch_id.into(),
            status: status.into(),
            message: message.into(),
        })),
    }
}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
