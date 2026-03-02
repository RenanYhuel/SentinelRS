use std::sync::Arc;

use sentinel_common::proto::Batch;

#[tonic::async_trait]
pub trait BrokerPublisher: Send + Sync {
    async fn publish(
        &self,
        batch: &Batch,
        signature: Option<&str>,
        key_id: Option<&str>,
    ) -> Result<(), BrokerError>;
}

#[tonic::async_trait]
impl<T: BrokerPublisher> BrokerPublisher for Arc<T> {
    async fn publish(
        &self,
        batch: &Batch,
        signature: Option<&str>,
        key_id: Option<&str>,
    ) -> Result<(), BrokerError> {
        (**self).publish(batch, signature, key_id).await
    }
}

#[derive(Debug)]
pub struct BrokerError(pub String);

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "broker: {}", self.0)
    }
}

impl std::error::Error for BrokerError {}
