use tokio_stream::StreamExt;
use tonic::Streaming;

use sentinel_common::proto::{
    server_message::Payload as ServerPayload, Batch, BatchAckStatus, ServerMessage,
};

use crate::buffer::Wal;
use prost::Message;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn receive_loop(
    mut inbound: Streaming<ServerMessage>,
    wal: Arc<Mutex<Wal>>,
) -> Result<(), RecvError> {
    while let Some(result) = inbound.next().await {
        let msg = result.map_err(|e| RecvError::Transport(e.to_string()))?;

        match msg.payload {
            Some(ServerPayload::BatchAck(ack)) => {
                let status =
                    BatchAckStatus::try_from(ack.status).unwrap_or(BatchAckStatus::BatchRejected);

                match status {
                    BatchAckStatus::BatchAccepted => {
                        tracing::debug!(target: "data", batch_id = %ack.batch_id, "Batch acknowledged");
                        ack_batch_in_wal(&wal, &ack.batch_id).await;
                    }
                    BatchAckStatus::BatchRejected => {
                        tracing::warn!(target: "data", batch_id = %ack.batch_id, reason = %ack.message, "Batch rejected");
                    }
                    BatchAckStatus::BatchRetry => {
                        tracing::warn!(target: "data", batch_id = %ack.batch_id, "Batch retry requested");
                    }
                }
            }
            Some(ServerPayload::HeartbeatPong(pong)) => {
                let latency = current_time_ms() - pong.server_time_ms;
                tracing::trace!(latency_ms = latency, "heartbeat pong");
            }
            Some(ServerPayload::ConfigUpdate(update)) => {
                tracing::info!(target: "cfg", version = update.version, "Config update received");
            }
            Some(ServerPayload::Command(cmd)) => {
                tracing::info!(target: "system", command_id = %cmd.command_id, "Remote command received");
            }
            Some(ServerPayload::Error(err)) => {
                if err.fatal {
                    tracing::error!(target: "conn", code = err.code, message = %err.message, "Fatal server error");
                    return Err(RecvError::FatalServerError(err.message));
                }
                tracing::warn!(target: "conn", code = err.code, message = %err.message, "Server error");
            }
            Some(ServerPayload::HandshakeAck(_)) => {
                tracing::warn!("unexpected handshake ack on active stream");
            }
            Some(ServerPayload::BootstrapResponse(_)) => {
                tracing::warn!("unexpected bootstrap response on active stream");
            }
            None => {}
        }
    }

    Err(RecvError::StreamEnded)
}

#[derive(Debug)]
pub enum RecvError {
    Transport(String),
    FatalServerError(String),
    StreamEnded,
}

impl std::fmt::Display for RecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "transport: {e}"),
            Self::FatalServerError(e) => write!(f, "fatal server error: {e}"),
            Self::StreamEnded => write!(f, "server stream ended"),
        }
    }
}

impl std::error::Error for RecvError {}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

async fn ack_batch_in_wal(wal: &Arc<Mutex<Wal>>, batch_id: &str) {
    let mut w = wal.lock().await;
    if let Ok(unacked) = w.iter_unacked() {
        for (record_id, data) in unacked {
            if let Ok(batch) = Batch::decode(data.as_slice()) {
                if batch.batch_id == batch_id {
                    w.ack(record_id);
                    tracing::trace!(target: "data", record_id, batch_id, "WAL record acked");
                    return;
                }
            }
        }
    }
    tracing::trace!(target: "data", batch_id, "Batch not found in WAL for ack");
}
