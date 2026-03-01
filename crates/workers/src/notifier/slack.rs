use reqwest::Client;

use super::channel::{Notifier, NotifyError};
use crate::alert::AlertEvent;

pub struct SlackNotifier {
    webhook_url: String,
    client: Client,
}

impl SlackNotifier {
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            client: Client::new(),
        }
    }
}

#[tonic::async_trait]
impl Notifier for SlackNotifier {
    fn name(&self) -> &str {
        "slack"
    }

    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let color = match event.severity {
            crate::alert::Severity::Info => "#36a64f",
            crate::alert::Severity::Warning => "#f2c744",
            crate::alert::Severity::Critical => "#d32f2f",
        };

        let status_emoji = match event.status {
            crate::alert::AlertStatus::Firing => ":fire:",
            crate::alert::AlertStatus::Resolved => ":white_check_mark:",
        };

        let payload = serde_json::json!({
            "attachments": [{
                "color": color,
                "title": format!("{} [{}] {}", status_emoji, event.severity_str(), event.rule_name),
                "fields": [
                    { "title": "Agent", "value": &event.agent_id, "short": true },
                    { "title": "Metric", "value": &event.metric_name, "short": true },
                    { "title": "Value", "value": format!("{:.2}", event.value), "short": true },
                    { "title": "Threshold", "value": format!("{:.2}", event.threshold), "short": true },
                ],
            }]
        });

        self.client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotifyError(e.to_string()))?
            .error_for_status()
            .map_err(|e| NotifyError(e.to_string()))?;

        Ok(())
    }
}
