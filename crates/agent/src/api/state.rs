use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AgentState {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    queue_length: AtomicU64,
    wal_size_bytes: AtomicU64,
    last_send_epoch: AtomicU64,
    batches_sent: AtomicU64,
    batches_failed: AtomicU64,
    ready: std::sync::atomic::AtomicBool,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                queue_length: AtomicU64::new(0),
                wal_size_bytes: AtomicU64::new(0),
                last_send_epoch: AtomicU64::new(0),
                batches_sent: AtomicU64::new(0),
                batches_failed: AtomicU64::new(0),
                ready: std::sync::atomic::AtomicBool::new(false),
            }),
        }
    }

    pub fn set_queue_length(&self, v: u64) {
        self.inner.queue_length.store(v, Ordering::Relaxed);
    }

    pub fn queue_length(&self) -> u64 {
        self.inner.queue_length.load(Ordering::Relaxed)
    }

    pub fn set_wal_size_bytes(&self, v: u64) {
        self.inner.wal_size_bytes.store(v, Ordering::Relaxed);
    }

    pub fn wal_size_bytes(&self) -> u64 {
        self.inner.wal_size_bytes.load(Ordering::Relaxed)
    }

    pub fn set_last_send_epoch(&self, v: u64) {
        self.inner.last_send_epoch.store(v, Ordering::Relaxed);
    }

    pub fn last_send_epoch(&self) -> u64 {
        self.inner.last_send_epoch.load(Ordering::Relaxed)
    }

    pub fn increment_batches_sent(&self) {
        self.inner.batches_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn batches_sent(&self) -> u64 {
        self.inner.batches_sent.load(Ordering::Relaxed)
    }

    pub fn increment_batches_failed(&self) {
        self.inner.batches_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn batches_failed(&self) -> u64 {
        self.inner.batches_failed.load(Ordering::Relaxed)
    }

    pub fn set_ready(&self, v: bool) {
        self.inner
            .ready
            .store(v, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_ready(&self) -> bool {
        self.inner
            .ready
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_updates() {
        let state = AgentState::new();
        state.set_queue_length(42);
        state.set_wal_size_bytes(1024);
        state.set_ready(true);
        state.increment_batches_sent();
        state.increment_batches_sent();
        state.increment_batches_failed();

        assert_eq!(state.queue_length(), 42);
        assert_eq!(state.wal_size_bytes(), 1024);
        assert!(state.is_ready());
        assert_eq!(state.batches_sent(), 2);
        assert_eq!(state.batches_failed(), 1);
    }

    #[test]
    fn clone_shares_state() {
        let a = AgentState::new();
        let b = a.clone();
        a.set_queue_length(99);
        assert_eq!(b.queue_length(), 99);
    }
}
