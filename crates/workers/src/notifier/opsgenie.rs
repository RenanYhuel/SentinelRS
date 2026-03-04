use reqwest::Client;

use super::channel::{Notifier, NotifyError};
use crate::alert::AlertEvent;

const ALERTS_API: &str = "https://api.opsgenie.com/v2/alerts";

pub struct OpsGenieNotifier {
    api_key: String,
    client: Client,
}

impl OpsGenieNotifier {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }
}

#[tonic::async_trait]
impl Notifier for OpsGenieNotifier {
    fn name(&self) -> &str {
        "opsgenie"
    }

    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let priority = match event.severity {
            crate::alert::Severity::Info => "P4",
            crate::alert::Severity::Warning => "P3",
            crate::alert::Severity::Critical => "P1",
        };

        match event.status {
            crate::alert::AlertStatus::Firing => self.create_alert(event, priority).await,
            crate::alert::AlertStatus::Resolved => self.close_alert(event).await,
        }
    }
}

impl OpsGenieNotifier {
    async fn create_alert(
        &self,
        event: &AlertEvent,
        priority: &str,
    ) -> Result<(), NotifyError> {
        let payload = serde_json::json!({
            "message": format!("[{}] {} - {}", event.severity_str(), event.rule_name, event.agent_id),
            "alias": event.fingerprint,
            "priority": priority,
            "source": "SentinelRS",
            "tags": ["sentinel", &event.severity_str().to_lowercase()],
            "details": {
                "agent": event.agent_id,
                "metric": event.metric_name,
                "value": format!("{:.2}", event.value),
                "threshold": format!("{:.2}", event.threshold),
                "rule_id": event.rule_id,
            }
        });

        self.client
            .post(ALERTS_API)
            .header("Authorization", format!("GenieKey {}", self.api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotifyError(e.to_string()))?
            .error_for_status()
            .map_err(|e| NotifyError(e.to_string()))?;

        Ok(())
    }

    async fn close_alert(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let url = format!(
            "{}/{}/close?identifierType=alias",
            ALERTS_API, event.fingerprint
        );

        let payload = serde_json::json!({
            "source": "SentinelRS",
            "note": format!("Resolved: {} on {}", event.rule_name, event.agent_id),
        });

        self.client
            .post(&url)
            .header("Authorization", format!("GenieKey {}", self.api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotifyError(e.to_string()))?
            .error_for_status()
            .map_err(|e| NotifyError(e.to_string()))?;

        Ok(())
    }
}
