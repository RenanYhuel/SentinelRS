use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::batch::BatchComposer;
use crate::buffer::Wal;

use super::sender::StreamSender;

const DRAIN_INTERVAL: Duration = Duration::from_secs(5);
const SAVE_INTERVAL_BATCHES: usize = 10;

pub async fn drain_loop(sender: StreamSender, wal: Arc<Mutex<Wal>>) {
    loop {
        let sent = drain_pending(&sender, &wal).await;
        if sent > 0 {
            tracing::debug!(target: "data", sent, "WAL batches pushed to stream");
        }
        tokio::time::sleep(DRAIN_INTERVAL).await;
    }
}

async fn drain_pending(sender: &StreamSender, wal: &Arc<Mutex<Wal>>) -> usize {
    let entries = {
        let w = wal.lock().await;
        match w.iter_unacked() {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(target: "data", error = %e, "WAL read failed");
                return 0;
            }
        }
    };

    if entries.is_empty() {
        return 0;
    }

    let mut sent = 0usize;

    for (_record_id, data) in &entries {
        let batch = match BatchComposer::decode_batch(data) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(target: "data", error = %e, "skipping undecodable WAL entry");
                continue;
            }
        };

        if sender.send_batch(batch).await.is_err() {
            tracing::warn!(target: "data", "stream channel closed, stopping drain");
            break;
        }
        sent += 1;

        if sent % SAVE_INTERVAL_BATCHES == 0 {
            tokio::task::yield_now().await;
        }
    }

    sent
}
