use async_nats::jetstream;
use prost::Message;

use super::publisher::{BrokerError, BrokerPublisher};
use sentinel_common::nats_config::subject_for_agent;
use sentinel_common::proto::Batch;

pub struct NatsPublisher {
    js: jetstream::Context,
}

impl NatsPublisher {
    pub fn new(js: jetstream::Context) -> Self {
        Self { js }
    }
}

#[tonic::async_trait]
impl BrokerPublisher for NatsPublisher {
    async fn publish(
        &self,
        batch: &Batch,
        signature: Option<&str>,
        key_id: Option<&str>,
    ) -> Result<(), BrokerError> {
        let subject = subject_for_agent(&batch.agent_id);
        let payload = batch.encode_to_vec();

        let mut headers = async_nats::HeaderMap::new();
        headers.insert("X-Agent-Id", batch.agent_id.as_str());
        headers.insert("X-Batch-Id", batch.batch_id.as_str());

        if let Some(sig) = signature {
            headers.insert("X-Signature", sig);
        }

        if let Some(kid) = key_id {
            headers.insert("X-Key-Id", kid);
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string();
        headers.insert("X-Received-At", now_ms.as_str());

        self.js
            .publish_with_headers(subject, headers, payload.into())
            .await
            .map_err(|e| BrokerError(e.to_string()))?
            .await
            .map_err(|e| BrokerError(e.to_string()))?;

        Ok(())
    }
}
