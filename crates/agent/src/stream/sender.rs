use tokio::sync::mpsc;

use sentinel_common::proto::{
    agent_message::Payload as AgentPayload, AgentMessage, HeartbeatPing, MetricsBatch, SystemStats,
};

use crate::security::HmacSigner;

use super::heartbeat::collect_system_stats;

#[derive(Clone)]
pub struct StreamSender {
    tx: mpsc::Sender<AgentMessage>,
    agent_id: String,
    signer: HmacSigner,
}

impl StreamSender {
    pub fn new(tx: mpsc::Sender<AgentMessage>, agent_id: String, signer: HmacSigner) -> Self {
        Self {
            tx,
            agent_id,
            signer,
        }
    }

    pub async fn send_batch(&self, batch: sentinel_common::proto::Batch) -> Result<(), SendError> {
        let canonical = sentinel_common::canonicalize::canonical_bytes(&batch);
        let signature = self.signer.sign_base64(&canonical);

        let metrics_batch = MetricsBatch {
            batch_id: batch.batch_id,
            seq_start: batch.seq_start,
            seq_end: batch.seq_end,
            created_at_ms: batch.created_at_ms,
            metrics: batch.metrics,
            meta: batch.meta,
            signature,
        };

        let msg = AgentMessage {
            payload: Some(AgentPayload::MetricsBatch(metrics_batch)),
        };

        self.tx
            .send(msg)
            .await
            .map_err(|_| SendError::ChannelClosed)
    }

    pub async fn send_heartbeat(&self) -> Result<(), SendError> {
        let stats = collect_system_stats();
        self.send_heartbeat_with_stats(stats).await
    }

    pub async fn send_heartbeat_with_stats(&self, stats: SystemStats) -> Result<(), SendError> {
        let msg = AgentMessage {
            payload: Some(AgentPayload::HeartbeatPing(HeartbeatPing {
                timestamp_ms: current_time_ms(),
                system_stats: Some(stats),
            })),
        };
        self.tx
            .send(msg)
            .await
            .map_err(|_| SendError::ChannelClosed)
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

#[derive(Debug)]
pub enum SendError {
    ChannelClosed,
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChannelClosed => write!(f, "stream sender channel closed"),
        }
    }
}

impl std::error::Error for SendError {}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
