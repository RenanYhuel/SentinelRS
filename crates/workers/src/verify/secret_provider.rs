#[tonic::async_trait]
pub trait SecretProvider: Send + Sync {
    async fn get_secret(&self, agent_id: &str) -> Option<Vec<u8>>;
}
