use sentinel_common::proto::Batch;

#[tonic::async_trait]
pub trait BrokerPublisher: Send + Sync {
    async fn publish(&self, batch: &Batch) -> Result<(), BrokerError>;
}

#[derive(Debug)]
pub struct BrokerError(pub String);

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "broker: {}", self.0)
    }
}

impl std::error::Error for BrokerError {}
