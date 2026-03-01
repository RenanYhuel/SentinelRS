use std::time::Duration;

pub struct RetryPolicy {
    pub max_attempts: Option<u32>,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: None,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.25,
        }
    }
}

impl RetryPolicy {
    pub fn with_max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = Some(n);
        self
    }

    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp = 2u64.saturating_pow(attempt);
        let delay_ms = (self.base_delay.as_millis() as u64).saturating_mul(exp);
        let capped = Duration::from_millis(delay_ms).min(self.max_delay);
        apply_jitter(capped, self.jitter_factor)
    }

    pub fn should_retry(&self, attempt: u32) -> bool {
        match self.max_attempts {
            Some(max) => attempt < max,
            None => true,
        }
    }
}

fn apply_jitter(base: Duration, factor: f64) -> Duration {
    if factor <= 0.0 {
        return base;
    }
    let ms = base.as_millis() as f64;
    let jitter_range = ms * factor;
    let random_offset = simple_random_f64() * jitter_range * 2.0 - jitter_range;
    let jittered = (ms + random_offset).max(0.0);
    Duration::from_millis(jittered as u64)
}

fn simple_random_f64() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 10000) as f64 / 10000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_jitter_policy() -> RetryPolicy {
        RetryPolicy {
            jitter_factor: 0.0,
            ..Default::default()
        }
    }

    #[test]
    fn exponential_backoff() {
        let policy = RetryPolicy {
            max_attempts: Some(5),
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            jitter_factor: 0.0,
        };
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(800));
    }

    #[test]
    fn capped_at_max_delay() {
        let mut policy = no_jitter_policy();
        policy.max_delay = Duration::from_secs(30);
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

    #[test]
    fn jitter_stays_within_bounds() {
        let policy = RetryPolicy {
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.25,
            max_attempts: None,
        };
        for _ in 0..20 {
            let d = policy.delay_for_attempt(0);
            assert!(d >= Duration::from_millis(750));
            assert!(d <= Duration::from_millis(1250));
        }
    }

    #[test]
    fn with_max_attempts_builder() {
        let policy = RetryPolicy::default().with_max_attempts(5);
        assert!(policy.should_retry(4));
        assert!(!policy.should_retry(5));
    }
}
