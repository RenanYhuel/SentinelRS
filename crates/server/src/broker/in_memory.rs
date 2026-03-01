use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::publisher::{BrokerError, BrokerPublisher};
use sentinel_common::proto::Batch;

#[derive(Clone)]
pub struct InMemoryBroker {
    batches: Arc<Mutex<Vec<Batch>>>,
    count: Arc<AtomicUsize>,
}

impl Default for InMemoryBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryBroker {
    pub fn new() -> Self {
        Self {
            batches: Arc::new(Mutex::new(Vec::new())),
            count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn published_count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    pub async fn published_batches(&self) -> Vec<Batch> {
        self.batches.lock().await.clone()
    }
}

#[tonic::async_trait]
impl BrokerPublisher for InMemoryBroker {
    async fn publish(
        &self,
        batch: &Batch,
        _signature: Option<&str>,
        _key_id: Option<&str>,
    ) -> Result<(), BrokerError> {
        self.batches.lock().await.push(batch.clone());
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_stores_batch() {
        let broker = InMemoryBroker::new();
        let batch = Batch {
            batch_id: "b-1".into(),
            ..Default::default()
        };
        broker.publish(&batch, None, None).await.unwrap();
        let stored = broker.published_batches().await;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].batch_id, "b-1");
    }
}
