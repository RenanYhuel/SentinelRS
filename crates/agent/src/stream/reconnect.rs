use std::time::Duration;

pub struct ReconnectPolicy {
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter_factor: f64,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.25,
        }
    }
}

impl ReconnectPolicy {
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp = 2u64.saturating_pow(attempt);
        let delay_ms = (self.base_delay.as_millis() as u64).saturating_mul(exp);
        let capped = Duration::from_millis(delay_ms).min(self.max_delay);
        apply_jitter(capped, self.jitter_factor)
    }
}

fn apply_jitter(base: Duration, factor: f64) -> Duration {
    if factor <= 0.0 {
        return base;
    }
    let ms = base.as_millis() as f64;
    let jitter_range = ms * factor;
    let offset = pseudo_random_f64() * jitter_range * 2.0 - jitter_range;
    let jittered = (ms + offset).max(0.0);
    Duration::from_millis(jittered as u64)
}

fn pseudo_random_f64() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 10000) as f64 / 10000.0
}
