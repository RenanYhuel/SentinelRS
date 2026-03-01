use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub struct WorkerMetrics {
    batches_processed: AtomicU64,
    batches_errors: AtomicU64,
    messages_acked: AtomicU64,
    messages_nacked: AtomicU64,
    rows_inserted: AtomicU64,
    alerts_fired: AtomicU64,
    notifications_sent: AtomicU64,
    notifications_failed: AtomicU64,
    processing_latency_sum_us: AtomicU64,
    processing_latency_count: AtomicU64,
    db_latency_sum_us: AtomicU64,
    db_latency_count: AtomicU64,
}

impl WorkerMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            batches_processed: AtomicU64::new(0),
            batches_errors: AtomicU64::new(0),
            messages_acked: AtomicU64::new(0),
            messages_nacked: AtomicU64::new(0),
            rows_inserted: AtomicU64::new(0),
            alerts_fired: AtomicU64::new(0),
            notifications_sent: AtomicU64::new(0),
            notifications_failed: AtomicU64::new(0),
            processing_latency_sum_us: AtomicU64::new(0),
            processing_latency_count: AtomicU64::new(0),
            db_latency_sum_us: AtomicU64::new(0),
            db_latency_count: AtomicU64::new(0),
        })
    }

    pub fn inc_batches_processed(&self) {
        self.batches_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_batches_errors(&self) {
        self.batches_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_messages_acked(&self) {
        self.messages_acked.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_messages_nacked(&self) {
        self.messages_nacked.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_rows_inserted(&self, count: u64) {
        self.rows_inserted.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_alerts_fired(&self, count: u64) {
        self.alerts_fired.fetch_add(count, Ordering::Relaxed);
    }

    pub fn inc_notifications_sent(&self) {
        self.notifications_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_notifications_failed(&self) {
        self.notifications_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_processing_latency(&self, start: Instant) {
        let us = start.elapsed().as_micros() as u64;
        self.processing_latency_sum_us
            .fetch_add(us, Ordering::Relaxed);
        self.processing_latency_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_db_latency(&self, start: Instant) {
        let us = start.elapsed().as_micros() as u64;
        self.db_latency_sum_us.fetch_add(us, Ordering::Relaxed);
        self.db_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn batches_processed_val(&self) -> u64 {
        self.batches_processed.load(Ordering::Relaxed)
    }

    pub fn batches_errors_val(&self) -> u64 {
        self.batches_errors.load(Ordering::Relaxed)
    }

    pub fn messages_acked_val(&self) -> u64 {
        self.messages_acked.load(Ordering::Relaxed)
    }

    pub fn messages_nacked_val(&self) -> u64 {
        self.messages_nacked.load(Ordering::Relaxed)
    }

    pub fn rows_inserted_val(&self) -> u64 {
        self.rows_inserted.load(Ordering::Relaxed)
    }

    pub fn alerts_fired_val(&self) -> u64 {
        self.alerts_fired.load(Ordering::Relaxed)
    }

    pub fn notifications_sent_val(&self) -> u64 {
        self.notifications_sent.load(Ordering::Relaxed)
    }

    pub fn notifications_failed_val(&self) -> u64 {
        self.notifications_failed.load(Ordering::Relaxed)
    }

    pub fn processing_latency_vals(&self) -> (u64, u64) {
        (
            self.processing_latency_sum_us.load(Ordering::Relaxed),
            self.processing_latency_count.load(Ordering::Relaxed),
        )
    }

    pub fn db_latency_vals(&self) -> (u64, u64) {
        (
            self.db_latency_sum_us.load(Ordering::Relaxed),
            self.db_latency_count.load(Ordering::Relaxed),
        )
    }
}

impl Default for WorkerMetrics {
    fn default() -> Self {
        Self {
            batches_processed: AtomicU64::new(0),
            batches_errors: AtomicU64::new(0),
            messages_acked: AtomicU64::new(0),
            messages_nacked: AtomicU64::new(0),
            rows_inserted: AtomicU64::new(0),
            alerts_fired: AtomicU64::new(0),
            notifications_sent: AtomicU64::new(0),
            notifications_failed: AtomicU64::new(0),
            processing_latency_sum_us: AtomicU64::new(0),
            processing_latency_count: AtomicU64::new(0),
            db_latency_sum_us: AtomicU64::new(0),
            db_latency_count: AtomicU64::new(0),
        }
    }
}
