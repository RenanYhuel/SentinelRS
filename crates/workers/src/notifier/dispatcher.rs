use sqlx::PgPool;

use super::discord::DiscordNotifier;
use super::dlq::DlqWriter;
use super::retry::RetryNotifier;
use super::slack::SlackNotifier;
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
        match cfg.ntype.as_str() {
            "discord" => {
                let url = extract_str(&cfg.config, "webhook_url")?;
                let notifier = RetryNotifier::new(DiscordNotifier::new(url), 2, 500)
                    .with_dlq(DlqWriter::new(self.dlq_pool()));
                notifier.send(event).await?;
            }
            "slack" => {
                let url = extract_str(&cfg.config, "webhook_url")?;
                let notifier = RetryNotifier::new(SlackNotifier::new(url), 2, 500)
                    .with_dlq(DlqWriter::new(self.dlq_pool()));
                notifier.send(event).await?;
            }
            "webhook" => {
                let url = extract_str(&cfg.config, "url")?;
                let secret = cfg
                    .config
                    .get("secret")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .as_bytes()
                    .to_vec();
                let notifier = RetryNotifier::new(WebhookNotifier::new(url, secret), 2, 500)
                    .with_dlq(DlqWriter::new(self.dlq_pool()));
                notifier.send(event).await?;
            }
            "smtp" => {
                tracing::warn!(target: "notify", "SMTP dispatch not yet wired (config: {})", cfg.name);
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
