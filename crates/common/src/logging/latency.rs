use std::time::Instant;

pub struct LatencyGuard {
    start: Instant,
    operation: &'static str,
}

impl LatencyGuard {
    pub fn new(operation: &'static str) -> Self {
        Self {
            start: Instant::now(),
            operation,
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    pub fn finish(self) -> u64 {
        let ms = self.elapsed_ms();
        tracing::debug!(
            target: "perf",
            operation = self.operation,
            latency_ms = ms,
            "Operation completed"
        );
        ms
    }

    pub fn finish_with_count(self, count: u64) -> u64 {
        let ms = self.elapsed_ms();
        tracing::debug!(
            target: "perf",
            operation = self.operation,
            latency_ms = ms,
            count,
            "Operation completed"
        );
        ms
    }
}

pub fn track(operation: &'static str) -> LatencyGuard {
    LatencyGuard::new(operation)
}

pub fn warn_slow(operation: &str, latency_ms: u64, threshold_ms: u64) {
    if latency_ms > threshold_ms {
        tracing::warn!(
            target: "system",
            operation,
            latency_ms,
            threshold_ms,
            "Slow operation detected"
        );
    }
}
