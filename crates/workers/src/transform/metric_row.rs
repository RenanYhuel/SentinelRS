use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRow {
    pub time_ms: i64,
    pub agent_id: String,
    pub name: String,
    pub labels: HashMap<String, String>,
    pub metric_type: String,
    pub value: Option<f64>,
    pub histogram_boundaries: Option<Vec<f64>>,
    pub histogram_counts: Option<Vec<u64>>,
    pub histogram_count: Option<u64>,
    pub histogram_sum: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauge_row_has_value() {
        let row = MetricRow {
            time_ms: 1000,
            agent_id: "a-1".into(),
            name: "cpu".into(),
            labels: HashMap::new(),
            metric_type: "gauge".into(),
            value: Some(42.0),
            histogram_boundaries: None,
            histogram_counts: None,
            histogram_count: None,
            histogram_sum: None,
        };
        assert_eq!(row.value, Some(42.0));
        assert!(row.histogram_boundaries.is_none());
    }

    #[test]
    fn histogram_row_has_buckets() {
        let row = MetricRow {
            time_ms: 2000,
            agent_id: "a-1".into(),
            name: "latency".into(),
            labels: HashMap::new(),
            metric_type: "histogram".into(),
            value: None,
            histogram_boundaries: Some(vec![10.0, 50.0, 100.0]),
            histogram_counts: Some(vec![5, 10, 3]),
            histogram_count: Some(18),
            histogram_sum: Some(1234.5),
        };
        assert!(row.histogram_boundaries.is_some());
        assert_eq!(row.histogram_count, Some(18));
    }
}
