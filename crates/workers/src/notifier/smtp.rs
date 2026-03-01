use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use super::channel::{Notifier, NotifyError};
use crate::alert::AlertEvent;

pub struct SmtpNotifier {
    from: String,
    to: String,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpNotifier {
    pub fn new(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        from: String,
        to: String,
    ) -> Self {
        let creds = Credentials::new(username.to_string(), password.to_string());
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .expect("valid SMTP host")
            .port(port)
            .credentials(creds)
            .build();
        Self {
            from,
            to,
            transport,
        }
    }
}

#[tonic::async_trait]
impl Notifier for SmtpNotifier {
    fn name(&self) -> &str {
        "smtp"
    }

    async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let subject = format!(
            "[SentinelRS] [{}] {} - {}",
            event.severity_str(),
            event.status_str(),
            event.rule_name
        );

        let body = format!(
            "Alert: {}\nAgent: {}\nMetric: {}\nValue: {:.2}\nThreshold: {:.2}\nStatus: {}",
            event.rule_name,
            event.agent_id,
            event.metric_name,
            event.value,
            event.threshold,
            event.status_str(),
        );

        let email = Message::builder()
            .from(
                self.from
                    .parse()
                    .map_err(|e: lettre::address::AddressError| NotifyError(e.to_string()))?,
            )
            .to(self
                .to
                .parse()
                .map_err(|e: lettre::address::AddressError| NotifyError(e.to_string()))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .map_err(|e| NotifyError(e.to_string()))?;

        self.transport
            .send(email)
            .await
            .map_err(|e| NotifyError(e.to_string()))?;

        Ok(())
    }
}
