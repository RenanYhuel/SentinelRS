use reqwest::Client;

use super::channel::{Notifier, NotifyError};
use crate::alert::AlertEvent;

pub struct TelegramNotifier {
    bot_token: String,
    chat_id: String,
    client: Client,
}

impl TelegramNotifier {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            bot_token,
            chat_id,
            client: Client::new(),
        }
    }
}

#[tonic::async_trait]
impl Notifier for TelegramNotifier {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let emoji = match event.status {
            crate::alert::AlertStatus::Firing => "\u{1F6A8}",
            crate::alert::AlertStatus::Resolved => "\u{2705}",
        };

        let text = format!(
            "{emoji} *\\[{severity}\\] {rule}*\n\
             Status: `{status}`\n\
             Agent: `{agent}`\n\
             Metric: `{metric}`\n\
             Value: `{value:.2}` \\(threshold: `{threshold:.2}`\\)",
            severity = event.severity_str(),
            rule = escape_markdown(&event.rule_name),
            status = event.status_str(),
            agent = escape_markdown(&event.agent_id),
            metric = escape_markdown(&event.metric_name),
            value = event.value,
            threshold = event.threshold,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);

        let payload = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "MarkdownV2",
        });

        self.client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotifyError(e.to_string()))?
            .error_for_status()
            .map_err(|e| NotifyError(e.to_string()))?;

        Ok(())
    }
}

fn escape_markdown(s: &str) -> String {
    let special = [
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
    ];
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if special.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
