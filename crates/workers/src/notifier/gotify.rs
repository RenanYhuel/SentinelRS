use reqwest::Client;

use super::channel::{Notifier, NotifyError};
use crate::alert::AlertEvent;

pub struct GotifyNotifier {
    server_url: String,
    token: String,
    client: Client,
}

impl GotifyNotifier {
    pub fn new(server_url: String, token: String) -> Self {
        let server_url = server_url.trim_end_matches('/').to_string();
        Self {
            server_url,
            token,
            client: Client::new(),
        }
    }
}

#[tonic::async_trait]
impl Notifier for GotifyNotifier {
    fn name(&self) -> &str {
        "gotify"
    }

    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let priority = match event.severity {
            crate::alert::Severity::Info => 2,
            crate::alert::Severity::Warning => 5,
            crate::alert::Severity::Critical => 8,
        };

        let emoji = match event.status {
            crate::alert::AlertStatus::Firing => "\u{1F6A8}",
            crate::alert::AlertStatus::Resolved => "\u{2705}",
        };

        let title = format!(
            "{emoji} [{severity}] {rule}",
            severity = event.severity_str(),
            rule = event.rule_name,
        );

        let message = format!(
            "Status: {status}\nAgent: {agent}\nMetric: {metric}\nValue: {value:.2}\nThreshold: {threshold:.2}",
            status = event.status_str(),
            agent = event.agent_id,
            metric = event.metric_name,
            value = event.value,
            threshold = event.threshold,
        );

        let url = format!("{}/message", self.server_url);

        let payload = serde_json::json!({
            "title": title,
            "message": message,
            "priority": priority,
            "extras": {
                "client::display": { "contentType": "text/plain" }
            }
        });

        self.client
            .post(&url)
            .header("X-Gotify-Key", &self.token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotifyError(e.to_string()))?
            .error_for_status()
            .map_err(|e| NotifyError(e.to_string()))?;

        Ok(())
    }
}
