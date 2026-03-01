use crate::alert::AlertEvent;

#[tonic::async_trait]
pub trait Notifier: Send + Sync {
    fn name(&self) -> &str;
    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError>;
}

#[derive(Debug)]
pub struct NotifyError(pub String);

impl std::fmt::Display for NotifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "notify: {}", self.0)
    }
}

impl std::error::Error for NotifyError {}
