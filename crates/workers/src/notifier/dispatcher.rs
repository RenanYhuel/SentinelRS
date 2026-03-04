use sqlx::PgPool;

use super::discord::DiscordNotifier;
use super::dlq::DlqWriter;
use super::gotify::GotifyNotifier;
use super::ntfy::NtfyNotifier;
use super::opsgenie::OpsGenieNotifier;
use super::pagerduty::PagerDutyNotifier;
use super::retry::RetryNotifier;
use super::slack::SlackNotifier;
use super::smtp::SmtpNotifier;
use super::teams::TeamsNotifier;
use super::telegram::TelegramNotifier;
use super::webhook::WebhookNotifier;
use crate::alert::AlertEvent;
use crate::storage::{NotifierConfigLoader, NotifierConfigRow};

pub struct Dispatcher {
    loader: NotifierConfigLoader,
}

impl Dispatcher {
    pub fn new(pool: PgPool) -> Self {
        Self {
            loader: NotifierConfigLoader::new(pool),
        }
    }

    pub async fn dispatch(&self, event: &AlertEvent, notifier_ids: &[String]) {
        let configs = match self.loader.load_by_ids(notifier_ids).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(target: "notify", error = %e, "failed to load notifier configs");
                return;
            }
        };

        for cfg in &configs {
            if let Err(e) = self.send_one(cfg, event).await {
                tracing::error!(
                    target: "notify",
                    notifier = %cfg.name,
                    error = %e,
                    "notification failed"
                );
            }
        }
    }

    async fn send_one(
        &self,
        cfg: &NotifierConfigRow,
        event: &AlertEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let dlq = || DlqWriter::new(self.dlq_pool());

        match cfg.ntype.as_str() {
            "discord" => {
                let url = extract_str(&cfg.config, "webhook_url")?;
                RetryNotifier::new(DiscordNotifier::new(url), 2, 500)
                    .with_dlq(dlq())
                    .send(event)
                    .await?;
            }
            "slack" => {
                let url = extract_str(&cfg.config, "webhook_url")?;
                RetryNotifier::new(SlackNotifier::new(url), 2, 500)
                    .with_dlq(dlq())
                    .send(event)
                    .await?;
            }
            "webhook" => {
                let url = extract_str(&cfg.config, "url")?;
                let secret = extract_bytes(&cfg.config, "secret");
                RetryNotifier::new(WebhookNotifier::new(url, secret), 2, 500)
                    .with_dlq(dlq())
                    .send(event)
                    .await?;
            }
            "smtp" => {
                let host = extract_str(&cfg.config, "host")?;
                let port = cfg
                    .config
                    .get("port")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(587) as u16;
                let username = extract_str(&cfg.config, "username").unwrap_or_default();
                let password = extract_str(&cfg.config, "password").unwrap_or_default();
                let from = extract_str(&cfg.config, "from")?;
                let to = extract_str(&cfg.config, "to")?;
                let notifier = SmtpNotifier::new(&host, port, &username, &password, from, to);
                RetryNotifier::new(notifier, 2, 500)
                    .with_dlq(dlq())
                    .send(event)
                    .await?;
            }
            "telegram" => {
                let bot_token = extract_str(&cfg.config, "bot_token")?;
                let chat_id = extract_str(&cfg.config, "chat_id")?;
                RetryNotifier::new(TelegramNotifier::new(bot_token, chat_id), 2, 500)
                    .with_dlq(dlq())
                    .send(event)
                    .await?;
            }
            "pagerduty" => {
                let routing_key = extract_str(&cfg.config, "routing_key")?;
                RetryNotifier::new(PagerDutyNotifier::new(routing_key), 2, 500)
                    .with_dlq(dlq())
                    .send(event)
                    .await?;
            }
            "teams" => {
                let url = extract_str(&cfg.config, "webhook_url")?;
                RetryNotifier::new(TeamsNotifier::new(url), 2, 500)
                    .with_dlq(dlq())
                    .send(event)
                    .await?;
            }
            "opsgenie" => {
                let api_key = extract_str(&cfg.config, "api_key")?;
                RetryNotifier::new(OpsGenieNotifier::new(api_key), 2, 500)
                    .with_dlq(dlq())
                    .send(event)
                    .await?;
            }
            "gotify" => {
                let server_url = extract_str(&cfg.config, "server_url")?;
                let token = extract_str(&cfg.config, "token")?;
                RetryNotifier::new(GotifyNotifier::new(server_url, token), 2, 500)
                    .with_dlq(dlq())
                    .send(event)
                    .await?;
            }
            "ntfy" => {
                let server_url = extract_str(&cfg.config, "server_url")?;
                let topic = extract_str(&cfg.config, "topic")?;
                let token = cfg
                    .config
                    .get("token")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                RetryNotifier::new(NtfyNotifier::new(server_url, topic, token), 2, 500)
                    .with_dlq(dlq())
                    .send(event)
                    .await?;
            }
            other => {
                tracing::warn!(target: "notify", ntype = other, "unknown notifier type");
            }
        }
        Ok(())
    }

    fn dlq_pool(&self) -> PgPool {
        self.loader.pool()
    }
}

fn extract_str(
    config: &serde_json::Value,
    key: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    config
        .get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| format!("missing '{key}' in notifier config").into())
}

fn extract_bytes(config: &serde_json::Value, key: &str) -> Vec<u8> {
    config
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .as_bytes()
        .to_vec()
}
