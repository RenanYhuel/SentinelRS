use reqwest::Client;

use super::channel::{Notifier, NotifyError};
use crate::alert::AlertEvent;

pub struct NtfyNotifier {
    server_url: String,
    topic: String,
    token: Option<String>,
    client: Client,
}

impl NtfyNotifier {
    pub fn new(server_url: String, topic: String, token: Option<String>) -> Self {
        let server_url = server_url.trim_end_matches('/').to_string();
        Self {
            server_url,
            topic,
            token,
            client: Client::new(),
        }
    }
}

#[tonic::async_trait]
impl Notifier for NtfyNotifier {
    fn name(&self) -> &str {
        "ntfy"
    }

    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let priority = match event.severity {
            crate::alert::Severity::Info => "3",
            crate::alert::Severity::Warning => "4",
            crate::alert::Severity::Critical => "5",
        };

        let tags = match event.status {
            crate::alert::AlertStatus::Firing => "rotating_light,warning",
            crate::alert::AlertStatus::Resolved => "white_check_mark,resolved",
        };

        let title = format!(
            "[{severity}] {rule}",
            severity = event.severity_str(),
            rule = event.rule_name,
        );

        let body = format!(
            "Status: {status}\nAgent: {agent}\nMetric: {metric}\nValue: {value:.2} (threshold: {threshold:.2})",
            status = event.status_str(),
            agent = event.agent_id,
            metric = event.metric_name,
            value = event.value,
            threshold = event.threshold,
        );

        let url = format!("{}/{}", self.server_url, self.topic);

        let mut req = self
            .client
            .post(&url)
            .header("Title", &title)
            .header("Priority", priority)
            .header("Tags", tags)
            .body(body);

        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }

        req.send()
            .await
            .map_err(|e| NotifyError(e.to_string()))?
            .error_for_status()
            .map_err(|e| NotifyError(e.to_string()))?;

        Ok(())
    }
}
