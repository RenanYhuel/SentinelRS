use sentinel_common::proto::{self, Batch, MetricType};

use super::metric_row::MetricRow;

pub fn transform_batch(batch: &Batch) -> Vec<MetricRow> {
    batch
        .metrics
        .iter()
        .map(|m| transform_metric(&batch.agent_id, m))
        .collect()
}

fn transform_metric(agent_id: &str, m: &proto::Metric) -> MetricRow {
    let metric_type = match MetricType::try_from(m.rtype) {
        Ok(MetricType::Gauge) => "gauge",
        Ok(MetricType::Counter) => "counter",
        Ok(MetricType::Histogram) => "histogram",
        _ => "unspecified",
    };

    let (value, hist_bounds, hist_counts, hist_count, hist_sum) = match &m.value {
        Some(proto::metric::Value::ValueDouble(v)) => (Some(*v), None, None, None, None),
        Some(proto::metric::Value::ValueInt(v)) => (Some(*v as f64), None, None, None, None),
        Some(proto::metric::Value::Histogram(h)) => (
            None,
            Some(h.boundaries.clone()),
            Some(h.counts.clone()),
            Some(h.count),
            Some(h.sum),
        ),
        None => (None, None, None, None, None),
    };

    let mut labels = m.labels.clone();
    let keys: Vec<String> = labels.keys().cloned().collect();
    let mut sorted_labels = std::collections::HashMap::new();
    let mut sorted_keys = keys;
    sorted_keys.sort();
    for k in sorted_keys {
        if let Some(v) = labels.remove(&k) {
            sorted_labels.insert(k, v);
        }
    }

    MetricRow {
        time_ms: m.timestamp_ms,
        agent_id: agent_id.to_string(),
        name: m.name.clone(),
        labels: sorted_labels,
        metric_type: metric_type.to_string(),
        value,
        histogram_boundaries: hist_bounds,
        histogram_counts: hist_counts,
        histogram_count: hist_count,
        histogram_sum: hist_sum,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn gauge_batch() -> Batch {
        Batch {
            agent_id: "agent-1".into(),
            batch_id: "b-1".into(),
            metrics: vec![proto::Metric {
                name: "cpu.usage".into(),
                labels: HashMap::from([
                    ("host".into(), "srv1".into()),
                    ("core".into(), "0".into()),
                ]),
                rtype: MetricType::Gauge as i32,
                value: Some(proto::metric::Value::ValueDouble(55.5)),
                timestamp_ms: 1000,
            }],
            ..Default::default()
        }
    }

    fn histogram_batch() -> Batch {
        Batch {
            agent_id: "agent-2".into(),
            batch_id: "b-2".into(),
            metrics: vec![proto::Metric {
                name: "request.latency".into(),
                labels: HashMap::new(),
                rtype: MetricType::Histogram as i32,
                value: Some(proto::metric::Value::Histogram(proto::Histogram {
                    boundaries: vec![10.0, 50.0, 100.0],
                    counts: vec![5, 10, 3],
                    count: 18,
                    sum: 1234.5,
                })),
                timestamp_ms: 2000,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn transform_gauge() {
        let rows = transform_batch(&gauge_batch());
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.agent_id, "agent-1");
        assert_eq!(row.name, "cpu.usage");
        assert_eq!(row.metric_type, "gauge");
        assert_eq!(row.value, Some(55.5));
        assert!(row.histogram_boundaries.is_none());
    }

    #[test]
    fn transform_histogram() {
        let rows = transform_batch(&histogram_batch());
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.metric_type, "histogram");
        assert!(row.value.is_none());
        assert_eq!(row.histogram_count, Some(18));
        assert_eq!(row.histogram_sum, Some(1234.5));
        assert_eq!(row.histogram_boundaries.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn labels_sorted() {
        let rows = transform_batch(&gauge_batch());
        let keys: Vec<&String> = rows[0].labels.keys().collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn int_value_cast_to_f64() {
        let batch = Batch {
            agent_id: "a".into(),
            batch_id: "b".into(),
            metrics: vec![proto::Metric {
                name: "count".into(),
                labels: HashMap::new(),
                rtype: MetricType::Counter as i32,
                value: Some(proto::metric::Value::ValueInt(42)),
                timestamp_ms: 3000,
            }],
            ..Default::default()
        };
        let rows = transform_batch(&batch);
        assert_eq!(rows[0].value, Some(42.0));
        assert_eq!(rows[0].metric_type, "counter");
    }

    #[test]
    fn empty_batch_returns_empty() {
        let batch = Batch::default();
        assert!(transform_batch(&batch).is_empty());
    }
}
