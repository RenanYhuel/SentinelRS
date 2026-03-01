use reqwest::Client;

use super::channel::{Notifier, NotifyError};
use crate::alert::AlertEvent;

pub struct DiscordNotifier {
    webhook_url: String,
    client: Client,
}

impl DiscordNotifier {
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            client: Client::new(),
        }
    }
}

#[tonic::async_trait]
impl Notifier for DiscordNotifier {
    fn name(&self) -> &str {
        "discord"
    }

    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let color = match event.severity {
            crate::alert::Severity::Info => 0x36a64f,
            crate::alert::Severity::Warning => 0xf2c744,
            crate::alert::Severity::Critical => 0xd32f2f,
        };

        let payload = serde_json::json!({
            "embeds": [{
                "title": format!("[{}] {}", event.severity_str(), event.rule_name),
                "color": color,
                "fields": [
                    { "name": "Status", "value": event.status_str(), "inline": true },
                    { "name": "Agent", "value": &event.agent_id, "inline": true },
                    { "name": "Metric", "value": &event.metric_name, "inline": true },
                    { "name": "Value", "value": format!("{:.2}", event.value), "inline": true },
                    { "name": "Threshold", "value": format!("{:.2}", event.threshold), "inline": true },
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
