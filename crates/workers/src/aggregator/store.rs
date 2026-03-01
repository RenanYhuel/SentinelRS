use dashmap::DashMap;
use std::sync::Arc;

use super::rolling::RollingSeries;

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct MetricKey {
    pub agent_id: String,
    pub name: String,
}

pub struct AggregatorStore {
    series: Arc<DashMap<MetricKey, RollingSeries>>,
    window_ms: i64,
}

impl AggregatorStore {
    pub fn new(window_ms: i64) -> Self {
        Self {
            series: Arc::new(DashMap::new()),
            window_ms,
        }
    }

    pub fn ingest(&self, agent_id: &str, name: &str, timestamp_ms: i64, value: f64) {
        let key = MetricKey {
            agent_id: agent_id.to_string(),
            name: name.to_string(),
        };
        self.series
            .entry(key)
            .or_insert_with(|| RollingSeries::new(self.window_ms))
            .push(timestamp_ms, value);
    }

    pub fn avg(&self, agent_id: &str, name: &str) -> Option<f64> {
        let key = MetricKey {
            agent_id: agent_id.to_string(),
            name: name.to_string(),
        };
        self.series.get(&key).and_then(|s| s.avg())
    }

    pub fn min(&self, agent_id: &str, name: &str) -> Option<f64> {
        let key = MetricKey {
            agent_id: agent_id.to_string(),
            name: name.to_string(),
        };
        self.series.get(&key).and_then(|s| s.min())
    }

    pub fn max(&self, agent_id: &str, name: &str) -> Option<f64> {
        let key = MetricKey {
            agent_id: agent_id.to_string(),
            name: name.to_string(),
        };
        self.series.get(&key).and_then(|s| s.max())
    }

    pub fn last(&self, agent_id: &str, name: &str) -> Option<f64> {
        let key = MetricKey {
            agent_id: agent_id.to_string(),
            name: name.to_string(),
        };
        self.series.get(&key).and_then(|s| s.last())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingest_and_query() {
        let store = AggregatorStore::new(5000);
        store.ingest("agent-1", "cpu", 1000, 40.0);
        store.ingest("agent-1", "cpu", 2000, 60.0);
        assert_eq!(store.avg("agent-1", "cpu"), Some(50.0));
        assert_eq!(store.min("agent-1", "cpu"), Some(40.0));
        assert_eq!(store.max("agent-1", "cpu"), Some(60.0));
    }

    #[test]
    fn missing_key_returns_none() {
        let store = AggregatorStore::new(5000);
        assert_eq!(store.avg("x", "y"), None);
    }

    #[test]
    fn separate_agents() {
        let store = AggregatorStore::new(10000);
        store.ingest("a1", "cpu", 100, 10.0);
        store.ingest("a2", "cpu", 100, 90.0);
        assert_eq!(store.avg("a1", "cpu"), Some(10.0));
        assert_eq!(store.avg("a2", "cpu"), Some(90.0));
    }
}
