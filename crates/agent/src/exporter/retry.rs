use std::time::Duration;

pub struct RetryPolicy {
    pub max_attempts: Option<u32>,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: None,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
        }
    }
}

impl RetryPolicy {
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp = 2u64.saturating_pow(attempt);
        let delay_ms = (self.base_delay.as_millis() as u64).saturating_mul(exp);
        let capped = Duration::from_millis(delay_ms).min(self.max_delay);
        capped
    }

    pub fn should_retry(&self, attempt: u32) -> bool {
        match self.max_attempts {
            Some(max) => attempt < max,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_backoff() {
        let policy = RetryPolicy {
            max_attempts: Some(5),
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
        };
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(800));
    }

    #[test]
    fn capped_at_max_delay() {
        let policy = RetryPolicy {
            max_attempts: None,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        };
        assert!(policy.delay_for_attempt(20) <= Duration::from_secs(30));
    }

    #[test]
    fn respects_max_attempts() {
        let policy = RetryPolicy {
            max_attempts: Some(3),
            ..Default::default()
        };
        assert!(policy.should_retry(0));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
    }

    #[test]
    fn unlimited_retries() {
        let policy = RetryPolicy::default();
        assert!(policy.should_retry(1000));
    }
}
