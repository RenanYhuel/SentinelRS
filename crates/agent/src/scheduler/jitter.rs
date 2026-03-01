use std::time::Duration;

pub fn apply_jitter(base: Duration, jitter_fraction: f64) -> Duration {
    if jitter_fraction <= 0.0 {
        return base;
    }
    let jitter_max = base.as_secs_f64() * jitter_fraction.clamp(0.0, 1.0);
    let offset = rand_f64() * jitter_max;
    Duration::from_secs_f64(base.as_secs_f64() + offset)
}

fn rand_f64() -> f64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut h = s.build_hasher();
    h.write_u64(0);
    (h.finish() as f64) / (u64::MAX as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_jitter_returns_base() {
        let base = Duration::from_secs(10);
        assert_eq!(apply_jitter(base, 0.0), base);
    }

    #[test]
    fn jitter_never_reduces_duration() {
        let base = Duration::from_secs(10);
        for _ in 0..100 {
            assert!(apply_jitter(base, 0.5) >= base);
        }
    }

    #[test]
    fn jitter_bounded_by_fraction() {
        let base = Duration::from_secs(10);
        for _ in 0..100 {
            let d = apply_jitter(base, 0.2);
            assert!(d.as_secs_f64() <= 12.0);
        }
    }
}
