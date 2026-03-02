use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    failure_count: AtomicU32,
    threshold: u32,
    reset_duration: Duration,
    last_failure: Mutex<Option<Instant>>,
    total_trips: AtomicU64,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, reset_duration: Duration) -> Arc<Self> {
        Arc::new(Self {
            failure_count: AtomicU32::new(0),
            threshold,
            reset_duration,
            last_failure: Mutex::new(None),
            total_trips: AtomicU64::new(0),
        })
    }

    pub async fn state(&self) -> State {
        let count = self.failure_count.load(Ordering::Relaxed);
        if count < self.threshold {
            return State::Closed;
        }
        let guard = self.last_failure.lock().await;
        match *guard {
            Some(ts) if ts.elapsed() >= self.reset_duration => State::HalfOpen,
            Some(_) => State::Open,
            None => State::Closed,
        }
    }

    pub async fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        *self.last_failure.lock().await = None;
    }

    pub async fn record_failure(&self) {
        let prev = self.failure_count.fetch_add(1, Ordering::Relaxed);
        *self.last_failure.lock().await = Some(Instant::now());
        if prev + 1 == self.threshold {
            self.total_trips.fetch_add(1, Ordering::Relaxed);
            tracing::warn!(
                target: "backpressure",
                threshold = self.threshold,
                "Circuit breaker OPEN — pausing processing"
            );
        }
    }

    pub async fn allow(&self) -> bool {
        match self.state().await {
            State::Closed => true,
            State::HalfOpen => {
                tracing::info!(target: "backpressure", "Circuit breaker half-open — allowing probe request");
                true
            }
            State::Open => false,
        }
    }

    pub fn total_trips(&self) -> u64 {
        self.total_trips.load(Ordering::Relaxed)
    }

    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn starts_closed() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(5));
        assert_eq!(cb.state().await, State::Closed);
        assert!(cb.allow().await);
    }

    #[tokio::test]
    async fn opens_after_threshold() {
        let cb = CircuitBreaker::new(2, Duration::from_secs(60));
        cb.record_failure().await;
        assert_eq!(cb.state().await, State::Closed);
        cb.record_failure().await;
        assert_eq!(cb.state().await, State::Open);
        assert!(!cb.allow().await);
        assert_eq!(cb.total_trips(), 1);
    }

    #[tokio::test]
    async fn resets_on_success() {
        let cb = CircuitBreaker::new(2, Duration::from_secs(60));
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, State::Open);
        cb.record_success().await;
        assert_eq!(cb.state().await, State::Closed);
        assert!(cb.allow().await);
    }

    #[tokio::test]
    async fn half_open_after_reset_duration() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(10));
        cb.record_failure().await;
        assert_eq!(cb.state().await, State::Open);
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(cb.state().await, State::HalfOpen);
        assert!(cb.allow().await);
    }
}
