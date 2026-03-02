use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;

#[derive(Debug)]
pub struct PipelineStats {
    decode: StageCounter,
    verify: StageCounter,
    dedup: StageCounter,
    transform: StageCounter,
    store: StageCounter,
    alert: StageCounter,
}

#[derive(Debug)]
struct StageCounter {
    latency_sum_us: AtomicU64,
    count: AtomicU64,
}

impl StageCounter {
    fn new() -> Self {
        Self {
            latency_sum_us: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    fn record(&self, start: Instant) {
        let us = start.elapsed().as_micros() as u64;
        self.latency_sum_us.fetch_add(us, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> StageSnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let sum = self.latency_sum_us.load(Ordering::Relaxed);
        StageSnapshot {
            count,
            total_us: sum,
            avg_us: if count > 0 { sum / count } else { 0 },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StageSnapshot {
    pub count: u64,
    pub total_us: u64,
    pub avg_us: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineSnapshot {
    pub decode: StageSnapshot,
    pub verify: StageSnapshot,
    pub dedup: StageSnapshot,
    pub transform: StageSnapshot,
    pub store: StageSnapshot,
    pub alert: StageSnapshot,
}

impl PipelineStats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            decode: StageCounter::new(),
            verify: StageCounter::new(),
            dedup: StageCounter::new(),
            transform: StageCounter::new(),
            store: StageCounter::new(),
            alert: StageCounter::new(),
        })
    }

    pub fn record_decode(&self, start: Instant) {
        self.decode.record(start);
    }

    pub fn record_verify(&self, start: Instant) {
        self.verify.record(start);
    }

    pub fn record_dedup(&self, start: Instant) {
        self.dedup.record(start);
    }

    pub fn record_transform(&self, start: Instant) {
        self.transform.record(start);
    }

    pub fn record_store(&self, start: Instant) {
        self.store.record(start);
    }

    pub fn record_alert(&self, start: Instant) {
        self.alert.record(start);
    }

    pub fn snapshot(&self) -> PipelineSnapshot {
        PipelineSnapshot {
            decode: self.decode.snapshot(),
            verify: self.verify.snapshot(),
            dedup: self.dedup.snapshot(),
            transform: self.transform.snapshot(),
            store: self.store.snapshot(),
            alert: self.alert.snapshot(),
        }
    }
}

pub fn render_pipeline_prometheus(stats: &Arc<PipelineStats>, worker_id: &str) -> String {
    use std::fmt::Write;

    let snap = stats.snapshot();
    let mut out = String::with_capacity(512);

    for (stage, s) in [
        ("decode", &snap.decode),
        ("verify", &snap.verify),
        ("dedup", &snap.dedup),
        ("transform", &snap.transform),
        ("store", &snap.store),
        ("alert", &snap.alert),
    ] {
        let _ = writeln!(out, "# TYPE sentinel_pipeline_{stage}_latency_us summary");
        let _ = writeln!(
            out,
            "sentinel_pipeline_{stage}_latency_us_sum{{worker=\"{worker_id}\"}} {}",
            s.total_us
        );
        let _ = writeln!(
            out,
            "sentinel_pipeline_{stage}_latency_us_count{{worker=\"{worker_id}\"}} {}",
            s.count
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn snapshot_starts_empty() {
        let stats = PipelineStats::new();
        let snap = stats.snapshot();
        assert_eq!(snap.decode.count, 0);
        assert_eq!(snap.store.avg_us, 0);
    }

    #[test]
    fn records_latency() {
        let stats = PipelineStats::new();
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(1));
        stats.record_decode(start);

        let snap = stats.snapshot();
        assert_eq!(snap.decode.count, 1);
        assert!(snap.decode.total_us > 0);
    }

    #[test]
    fn prometheus_output() {
        let stats = PipelineStats::new();
        let start = Instant::now();
        stats.record_verify(start);

        let out = render_pipeline_prometheus(&stats, "test-worker");
        assert!(out.contains("sentinel_pipeline_verify_latency_us_count{worker=\"test-worker\"}"));
        assert!(out.contains("sentinel_pipeline_decode_latency_us_sum"));
    }
}
