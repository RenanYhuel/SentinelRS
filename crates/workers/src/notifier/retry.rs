use super::channel::{Notifier, NotifyError};
use super::dlq::DlqWriter;
use crate::alert::AlertEvent;

pub struct RetryNotifier<N: Notifier> {
    inner: N,
    max_retries: u32,
    base_delay_ms: u64,
    dlq: Option<DlqWriter>,
}

impl<N: Notifier> RetryNotifier<N> {
    pub fn new(inner: N, max_retries: u32, base_delay_ms: u64) -> Self {
        Self {
            inner,
            max_retries,
            base_delay_ms,
            dlq: None,
        }
    }

    pub fn with_dlq(mut self, dlq: DlqWriter) -> Self {
        self.dlq = Some(dlq);
        self
    }

    pub async fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        let mut last_err = None;

        for attempt in 0..=self.max_retries {
            match self.inner.send(event).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_err = Some(e);
                    if attempt < self.max_retries {
                        let delay = self.base_delay_ms * 2u64.pow(attempt);
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        let err = last_err.unwrap();

        if let Some(ref dlq) = self.dlq {
            let payload = serde_json::to_value(event).unwrap_or_default();
            if let Err(dlq_err) = dlq
                .insert(
                    &event.id,
                    self.inner.name(),
                    &payload,
                    &err.to_string(),
                    self.max_retries + 1,
                )
                .await
            {
                tracing::error!(error = %dlq_err, "failed to write to DLQ");
            }
        }

        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::AlertStatus;
    use crate::alert::Severity;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct FailingNotifier {
        fail_count: AtomicU32,
        max_failures: u32,
    }

    impl FailingNotifier {
        fn new(max_failures: u32) -> Self {
            Self {
                fail_count: AtomicU32::new(0),
                max_failures,
            }
        }
    }

    #[tonic::async_trait]
    impl Notifier for FailingNotifier {
        fn name(&self) -> &str {
            "test"
        }

        async fn send(&self, _event: &AlertEvent) -> Result<(), NotifyError> {
            let count = self.fail_count.fetch_add(1, Ordering::SeqCst);
            if count < self.max_failures {
                Err(NotifyError(format!("fail #{}", count + 1)))
            } else {
                Ok(())
            }
        }
    }

    fn sample_event() -> AlertEvent {
        AlertEvent {
            id: "evt-1".into(),
            fingerprint: "fp-1".into(),
            rule_id: "r-1".into(),
            rule_name: "test rule".into(),
            agent_id: "agent-1".into(),
            metric_name: "cpu".into(),
            severity: Severity::Warning,
            status: AlertStatus::Firing,
            value: 90.0,
            threshold: 80.0,
            fired_at_ms: 1000,
            resolved_at_ms: None,
            annotations: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn succeeds_on_first_try() {
        let inner = FailingNotifier::new(0);
        let retry = RetryNotifier::new(inner, 3, 1);
        assert!(retry.send(&sample_event()).await.is_ok());
    }

    #[tokio::test]
    async fn succeeds_after_retries() {
        let inner = FailingNotifier::new(2);
        let retry = RetryNotifier::new(inner, 3, 1);
        assert!(retry.send(&sample_event()).await.is_ok());
    }

    #[tokio::test]
    async fn fails_after_max_retries() {
        let inner = FailingNotifier::new(10);
        let retry = RetryNotifier::new(inner, 2, 1);
        assert!(retry.send(&sample_event()).await.is_err());
    }
}
