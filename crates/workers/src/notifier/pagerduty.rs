use reqwest::Client;

use super::channel::{Notifier, NotifyError};
use crate::alert::AlertEvent;

const EVENTS_API: &str = "https://events.pagerduty.com/v2/enqueue";

pub struct PagerDutyNotifier {
    routing_key: String,
    client: Client,
}

impl PagerDutyNotifier {
    pub fn new(routing_key: String) -> Self {
        Self {
            routing_key,
            client: Client::new(),
        }
    }
}

#[tonic::async_trait]
impl Notifier for PagerDutyNotifier {
    fn name(&self) -> &str {
        "pagerduty"
    }

    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let event_action = match event.status {
            crate::alert::AlertStatus::Firing => "trigger",
            crate::alert::AlertStatus::Resolved => "resolve",
        };

        let severity = match event.severity {
            crate::alert::Severity::Info => "info",
            crate::alert::Severity::Warning => "warning",
            crate::alert::Severity::Critical => "critical",
        };

        let payload = serde_json::json!({
            "routing_key": self.routing_key,
            "event_action": event_action,
            "dedup_key": event.fingerprint,
            "payload": {
                "summary": format!("[{}] {} - {}", event.severity_str(), event.rule_name, event.agent_id),
                "source": event.agent_id,
                "severity": severity,
                "component": event.metric_name,
                "custom_details": {
                    "metric": event.metric_name,
                    "value": event.value,
                    "threshold": event.threshold,
                    "rule_id": event.rule_id,
                    "fired_at_ms": event.fired_at_ms,
                }
            }
        });

        self.client
            .post(EVENTS_API)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotifyError(e.to_string()))?
            .error_for_status()
            .map_err(|e| NotifyError(e.to_string()))?;

        Ok(())
    }
}
