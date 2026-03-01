use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub struct ServerMetrics {
    grpc_requests_total: AtomicU64,
    grpc_errors_total: AtomicU64,
    rest_requests_total: AtomicU64,
    registrations_total: AtomicU64,
    pushes_accepted_total: AtomicU64,
    pushes_rejected_total: AtomicU64,
    heartbeats_total: AtomicU64,
    key_rotations_total: AtomicU64,
    broker_publish_errors_total: AtomicU64,
    grpc_latency_sum_us: AtomicU64,
    grpc_latency_count: AtomicU64,
}

impl ServerMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn inc_grpc_requests(&self) {
        self.grpc_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_grpc_errors(&self) {
        self.grpc_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_rest_requests(&self) {
        self.rest_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_registrations(&self) {
        self.registrations_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_pushes_accepted(&self) {
        self.pushes_accepted_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_pushes_rejected(&self) {
        self.pushes_rejected_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_heartbeats(&self) {
        self.heartbeats_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_key_rotations(&self) {
        self.key_rotations_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_broker_publish_errors(&self) {
        self.broker_publish_errors_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_grpc_latency(&self, start: Instant) {
        let us = start.elapsed().as_micros() as u64;
        self.grpc_latency_sum_us.fetch_add(us, Ordering::Relaxed);
        self.grpc_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn grpc_requests_total(&self) -> u64 {
        self.grpc_requests_total.load(Ordering::Relaxed)
    }

    pub fn grpc_errors_total(&self) -> u64 {
        self.grpc_errors_total.load(Ordering::Relaxed)
    }

    pub fn rest_requests_total(&self) -> u64 {
        self.rest_requests_total.load(Ordering::Relaxed)
    }

    pub fn registrations_total(&self) -> u64 {
        self.registrations_total.load(Ordering::Relaxed)
    }

    pub fn pushes_accepted_total(&self) -> u64 {
        self.pushes_accepted_total.load(Ordering::Relaxed)
    }

    pub fn pushes_rejected_total(&self) -> u64 {
        self.pushes_rejected_total.load(Ordering::Relaxed)
    }

    pub fn heartbeats_total(&self) -> u64 {
        self.heartbeats_total.load(Ordering::Relaxed)
    }

    pub fn key_rotations_total(&self) -> u64 {
        self.key_rotations_total.load(Ordering::Relaxed)
    }

    pub fn broker_publish_errors_total(&self) -> u64 {
        self.broker_publish_errors_total.load(Ordering::Relaxed)
    }

    pub fn grpc_latency_vals(&self) -> (u64, u64) {
        (
            self.grpc_latency_sum_us.load(Ordering::Relaxed),
            self.grpc_latency_count.load(Ordering::Relaxed),
        )
    }
}

impl Default for ServerMetrics {
    fn default() -> Self {
        Self {
            grpc_requests_total: AtomicU64::new(0),
            grpc_errors_total: AtomicU64::new(0),
            rest_requests_total: AtomicU64::new(0),
            registrations_total: AtomicU64::new(0),
            pushes_accepted_total: AtomicU64::new(0),
            pushes_rejected_total: AtomicU64::new(0),
            heartbeats_total: AtomicU64::new(0),
            key_rotations_total: AtomicU64::new(0),
            broker_publish_errors_total: AtomicU64::new(0),
            grpc_latency_sum_us: AtomicU64::new(0),
            grpc_latency_count: AtomicU64::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn counters_increment() {
        let m = ServerMetrics::new();
        m.inc_grpc_requests();
        m.inc_grpc_requests();
        m.inc_grpc_errors();
        m.inc_registrations();
        m.inc_pushes_accepted();
        m.inc_pushes_rejected();
        m.inc_heartbeats();
        m.inc_key_rotations();
        m.inc_broker_publish_errors();
        m.inc_rest_requests();

        assert_eq!(m.grpc_requests_total(), 2);
        assert_eq!(m.grpc_errors_total(), 1);
        assert_eq!(m.registrations_total(), 1);
        assert_eq!(m.pushes_accepted_total(), 1);
        assert_eq!(m.pushes_rejected_total(), 1);
        assert_eq!(m.heartbeats_total(), 1);
        assert_eq!(m.key_rotations_total(), 1);
        assert_eq!(m.broker_publish_errors_total(), 1);
        assert_eq!(m.rest_requests_total(), 1);
    }

    #[test]
    fn latency_recording() {
        let m = ServerMetrics::new();
        let start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(1));
        m.record_grpc_latency(start);
        let (sum, count) = m.grpc_latency_vals();
        assert!(sum > 0);
        assert_eq!(count, 1);
    }
}
