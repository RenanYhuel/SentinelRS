use reqwest::Client;

use super::channel::{Notifier, NotifyError};
use crate::alert::AlertEvent;

pub struct TeamsNotifier {
    webhook_url: String,
    client: Client,
}

impl TeamsNotifier {
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            client: Client::new(),
        }
    }
}

#[tonic::async_trait]
impl Notifier for TeamsNotifier {
    fn name(&self) -> &str {
        "teams"
    }

    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let color = match event.severity {
            crate::alert::Severity::Info => "Good",
            crate::alert::Severity::Warning => "Warning",
            crate::alert::Severity::Critical => "Attention",
        };

        let status_emoji = match event.status {
            crate::alert::AlertStatus::Firing => "\u{1F525}",
            crate::alert::AlertStatus::Resolved => "\u{2705}",
        };

        let payload = serde_json::json!({
            "type": "message",
            "attachments": [{
                "contentType": "application/vnd.microsoft.card.adaptive",
                "content": {
                    "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                    "type": "AdaptiveCard",
                    "version": "1.4",
                    "body": [
                        {
                            "type": "TextBlock",
                            "size": "Large",
                            "weight": "Bolder",
                            "text": format!("{status_emoji} [{severity}] {rule}",
                                severity = event.severity_str(),
                                rule = event.rule_name),
                            "style": color,
                        },
                        {
                            "type": "FactSet",
                            "facts": [
                                { "title": "Status", "value": event.status_str() },
                                { "title": "Agent", "value": &event.agent_id },
                                { "title": "Metric", "value": &event.metric_name },
                                { "title": "Value", "value": format!("{:.2}", event.value) },
                                { "title": "Threshold", "value": format!("{:.2}", event.threshold) },
                            ]
                        }
                    ]
                }
            }]
        });

        self.client
            .post(&self.webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotifyError(e.to_string()))?
            .error_for_status()
            .map_err(|e| NotifyError(e.to_string()))?;

        Ok(())
    }
}
