use reqwest::Client;

use crate::alert::AlertEvent;
use super::channel::{Notifier, NotifyError};
use super::signer::sign_payload;

pub struct WebhookNotifier {
    url: String,
    secret: Vec<u8>,
    client: Client,
}

impl WebhookNotifier {
    pub fn new(url: String, secret: Vec<u8>) -> Self {
        Self {
            url,
            secret,
            client: Client::new(),
        }
    }
}

#[tonic::async_trait]
impl Notifier for WebhookNotifier {
    fn name(&self) -> &str {
        "webhook"
    }

    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let body = serde_json::to_vec(event).map_err(|e| NotifyError(e.to_string()))?;
        let signature = sign_payload(&self.secret, &body);

        self.client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("X-Sentinel-Signature", &signature)
            .body(body)
            .send()
            .await
            .map_err(|e| NotifyError(e.to_string()))?
            .error_for_status()
            .map_err(|e| NotifyError(e.to_string()))?;

        Ok(())
    }
}
