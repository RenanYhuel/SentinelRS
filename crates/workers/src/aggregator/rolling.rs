use std::collections::VecDeque;

pub struct RollingSeries {
    window_ms: i64,
    samples: VecDeque<(i64, f64)>,
}

impl RollingSeries {
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms,
            samples: VecDeque::new(),
        }
    }

    pub fn push(&mut self, timestamp_ms: i64, value: f64) {
        self.samples.push_back((timestamp_ms, value));
        self.evict(timestamp_ms);
    }

    pub fn avg(&self) -> Option<f64> {
        if self.samples.is_empty() {
            return None;
        }
        let sum: f64 = self.samples.iter().map(|(_, v)| v).sum();
        Some(sum / self.samples.len() as f64)
    }

    pub fn min(&self) -> Option<f64> {
        self.samples.iter().map(|(_, v)| *v).reduce(f64::min)
    }

    pub fn max(&self) -> Option<f64> {
        self.samples.iter().map(|(_, v)| *v).reduce(f64::max)
    }

    pub fn last(&self) -> Option<f64> {
        self.samples.back().map(|(_, v)| *v)
    }

    pub fn count(&self) -> usize {
        self.samples.len()
    }

    fn evict(&mut self, now_ms: i64) {
        let cutoff = now_ms - self.window_ms;
        while let Some(&(ts, _)) = self.samples.front() {
            if ts < cutoff {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avg_of_window() {
        let mut s = RollingSeries::new(1000);
        s.push(100, 10.0);
        s.push(200, 20.0);
        s.push(300, 30.0);
        assert_eq!(s.avg(), Some(20.0));
    }

    #[test]
    fn evicts_old_samples() {
        let mut s = RollingSeries::new(500);
        s.push(100, 10.0);
        s.push(200, 20.0);
        s.push(700, 30.0);
        assert_eq!(s.count(), 2);
        assert_eq!(s.avg(), Some(25.0));
    }

    #[test]
    fn min_max() {
        let mut s = RollingSeries::new(10000);
        s.push(1, 5.0);
        s.push(2, 15.0);
        s.push(3, 10.0);
        assert_eq!(s.min(), Some(5.0));
        assert_eq!(s.max(), Some(15.0));
    }

    #[test]
    fn empty_returns_none() {
        let s = RollingSeries::new(1000);
        assert_eq!(s.avg(), None);
        assert_eq!(s.min(), None);
        assert_eq!(s.max(), None);
        assert_eq!(s.last(), None);
    }
}
