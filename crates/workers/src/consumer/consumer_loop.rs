use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_nats::jetstream::consumer::PullConsumer;
use tokio_util::sync::CancellationToken;

use super::handler::{decode_batch, extract_header, pull_batch};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct ConsumerLoop {
    consumer: PullConsumer,
    batch_size: usize,
    cancel: CancellationToken,
    in_flight: Arc<AtomicU64>,
}

impl ConsumerLoop {
    pub fn new(consumer: PullConsumer, batch_size: usize, cancel: CancellationToken) -> Self {
        Self {
            consumer,
            batch_size,
            cancel,
            in_flight: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn in_flight(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.in_flight)
    }

    pub async fn run<F, Fut>(&self, on_batch: F) -> Result<(), BoxError>
    where
        F: Fn(sentinel_common::proto::Batch, Option<String>) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        loop {
            if self.cancel.is_cancelled() {
                tracing::info!(target: "work", "Shutdown signal received, draining in-flight messages");
                self.drain().await;
                return Ok(());
            }

            let messages = tokio::select! {
                _ = self.cancel.cancelled() => {
                    self.drain().await;
                    return Ok(());
                }
                result = pull_batch(&self.consumer, self.batch_size) => result?,
            };

            if messages.is_empty() {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }

            for msg in messages {
                self.in_flight.fetch_add(1, Ordering::Relaxed);
                let batch_id = extract_header(&msg, "X-Batch-Id");

                match decode_batch(&msg) {
                    Ok(batch) => {
                        if let Err(e) = on_batch(batch, batch_id).await {
                            tracing::error!(target: "work", error = %e, "Batch processing failed, nacking");
                            if let Err(ne) = msg.ack().await {
                                tracing::error!(target: "work", error = %ne, "Nack failed");
                            }
                            self.in_flight.fetch_sub(1, Ordering::Relaxed);
                            continue;
                        }
                        if let Err(e) = msg.ack().await {
                            tracing::error!(target: "work", error = %e, "Ack failed");
                        }
                    }
                    Err(e) => {
                        tracing::error!(target: "work", error = %e, "Decode failed, acking to discard");
                        let _ = msg.ack().await;
                    }
                }
                self.in_flight.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }

    async fn drain(&self) {
        let remaining = self.in_flight.load(Ordering::Relaxed);
        if remaining > 0 {
            tracing::info!(target: "work", remaining, "Waiting for in-flight messages");
            for _ in 0..50 {
                if self.in_flight.load(Ordering::Relaxed) == 0 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
        tracing::info!(target: "work", "Consumer loop drained");
    }
}
