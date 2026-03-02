const WINDOW_SIZE: usize = 128;

#[derive(Debug, Clone)]
pub struct LatencyTracker {
    samples: Vec<i64>,
    cursor: usize,
    count: u64,
    min: i64,
    max: i64,
    sum: i64,
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self {
            samples: Vec::with_capacity(WINDOW_SIZE),
            cursor: 0,
            count: 0,
            min: i64::MAX,
            max: i64::MIN,
            sum: 0,
        }
    }

    pub fn record(&mut self, latency_ms: i64) {
        if self.samples.len() < WINDOW_SIZE {
            self.samples.push(latency_ms);
        } else {
            self.sum -= self.samples[self.cursor];
            self.samples[self.cursor] = latency_ms;
        }
        self.cursor = (self.cursor + 1) % WINDOW_SIZE;
        self.count += 1;
        self.sum += latency_ms;

        if latency_ms < self.min {
            self.min = latency_ms;
        }
        if latency_ms > self.max {
            self.max = latency_ms;
        }
    }

    pub fn avg(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.sum as f64 / self.samples.len() as f64
    }

    pub fn min(&self) -> i64 {
        if self.count == 0 {
            return 0;
        }
        self.min
    }

    pub fn max(&self) -> i64 {
        if self.count == 0 {
            return 0;
        }
        self.max
    }

    pub fn percentile(&self, p: f64) -> i64 {
        if self.samples.is_empty() {
            return 0;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    pub fn p50(&self) -> i64 {
        self.percentile(50.0)
    }

    pub fn p95(&self) -> i64 {
        self.percentile(95.0)
    }

    pub fn p99(&self) -> i64 {
        self.percentile(99.0)
    }

    pub fn jitter(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let avg = self.avg();
        let variance: f64 = self
            .samples
            .iter()
            .map(|&s| (s as f64 - avg).powi(2))
            .sum::<f64>()
            / self.samples.len() as f64;
        variance.sqrt()
    }

    pub fn total_count(&self) -> u64 {
        self.count
    }

    pub fn snapshot(&self) -> LatencySnapshot {
        LatencySnapshot {
            min_ms: self.min(),
            max_ms: self.max(),
            avg_ms: self.avg(),
            p50_ms: self.p50(),
            p95_ms: self.p95(),
            p99_ms: self.p99(),
            jitter_ms: self.jitter(),
            sample_count: self.total_count(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LatencySnapshot {
    pub min_ms: i64,
    pub max_ms: i64,
    pub avg_ms: f64,
    pub p50_ms: i64,
    pub p95_ms: i64,
    pub p99_ms: i64,
    pub jitter_ms: f64,
    pub sample_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_stats() {
        let mut t = LatencyTracker::new();
        for i in 1..=100 {
            t.record(i);
        }
        assert_eq!(t.min(), 1);
        assert_eq!(t.max(), 100);
        assert_eq!(t.total_count(), 100);
        assert!(t.avg() > 49.0 && t.avg() < 51.0);
        assert!(t.p50() >= 49 && t.p50() <= 51);
        assert!(t.p95() >= 94 && t.p95() <= 96);
    }

    #[test]
    fn empty_tracker() {
        let t = LatencyTracker::new();
        assert_eq!(t.avg(), 0.0);
        assert_eq!(t.min(), 0);
        assert_eq!(t.max(), 0);
        assert_eq!(t.jitter(), 0.0);
    }
}
