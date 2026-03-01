use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<RateLimiterInner>,
}

struct RateLimiterInner {
    max_rps: u64,
    count: AtomicU64,
    window_start: Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(max_rps: u64) -> Self {
        Self {
            inner: Arc::new(RateLimiterInner {
                max_rps,
                count: AtomicU64::new(0),
                window_start: Mutex::new(Instant::now()),
            }),
        }
    }

    pub async fn check(&self) -> bool {
        let mut start = self.inner.window_start.lock().await;
        if start.elapsed().as_secs() >= 1 {
            *start = Instant::now();
            self.inner.count.store(1, Ordering::Relaxed);
            return true;
        }
        let prev = self.inner.count.fetch_add(1, Ordering::Relaxed);
        prev < self.inner.max_rps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn allows_within_limit() {
        let rl = RateLimiter::new(5);
        for _ in 0..5 {
            assert!(rl.check().await);
        }
    }

    #[tokio::test]
    async fn rejects_over_limit() {
        let rl = RateLimiter::new(2);
        assert!(rl.check().await);
        assert!(rl.check().await);
        assert!(!rl.check().await);
    }
}
