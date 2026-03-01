use async_nats::jetstream::consumer::PullConsumer;

use super::handler::{decode_batch, extract_header, pull_batch};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct ConsumerLoop {
    consumer: PullConsumer,
    batch_size: usize,
}

impl ConsumerLoop {
    pub fn new(consumer: PullConsumer, batch_size: usize) -> Self {
        Self {
            consumer,
            batch_size,
        }
    }

    pub async fn run<F, Fut>(&self, on_batch: F) -> Result<(), BoxError>
    where
        F: Fn(sentinel_common::proto::Batch, Option<String>) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        loop {
            let messages = pull_batch(&self.consumer, self.batch_size).await?;

            if messages.is_empty() {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }

            for msg in messages {
                let batch_id = extract_header(&msg, "X-Batch-Id");

                match decode_batch(&msg) {
                    Ok(batch) => {
                        if let Err(e) = on_batch(batch, batch_id).await {
                            tracing::error!(error = %e, "processing batch failed, nacking");
                            if let Err(ne) = msg.ack().await {
                                tracing::error!(error = %ne, "nack failed");
                            }
                            continue;
                        }
                        if let Err(e) = msg.ack().await {
                            tracing::error!(error = %e, "ack failed");
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "decode failed, acking to discard");
                        let _ = msg.ack().await;
                    }
                }
            }
        }
    }
}
