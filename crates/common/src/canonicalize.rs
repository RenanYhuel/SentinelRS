use prost::Message;

use crate::proto::Batch;
use crate::proto::Metric;

fn canonicalize_metric(m: &Metric) -> Metric {
    let mut sorted_labels: Vec<(String, String)> = m.labels.clone().into_iter().collect();
    sorted_labels.sort_by(|a, b| a.0.cmp(&b.0));
    Metric {
        name: m.name.clone(),
        labels: sorted_labels.into_iter().collect(),
        rtype: m.rtype,
        value: m.value.clone(),
        timestamp_ms: m.timestamp_ms,
    }
}

pub fn canonical_bytes(batch: &Batch) -> Vec<u8> {
    let canonical = Batch {
        agent_id: batch.agent_id.clone(),
        batch_id: batch.batch_id.clone(),
        seq_start: batch.seq_start,
        seq_end: batch.seq_end,
        created_at_ms: batch.created_at_ms,
        metrics: batch.metrics.iter().map(canonicalize_metric).collect(),
        meta: {
            let mut sorted: Vec<(String, String)> = batch.meta.clone().into_iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            sorted.into_iter().collect()
        },
    };
    canonical.encode_to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_batch(labels_order: Vec<(&str, &str)>) -> Batch {
        let labels: HashMap<String, String> = labels_order
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Batch {
            agent_id: "agent-1".into(),
            batch_id: "b-1".into(),
            seq_start: 0,
            seq_end: 1,
            created_at_ms: 1000,
            metrics: vec![Metric {
                name: "cpu.usage".into(),
                labels,
                rtype: 1,
                value: Some(crate::proto::metric::Value::ValueDouble(42.0)),
                timestamp_ms: 1000,
            }],
            meta: HashMap::new(),
        }
    }

    #[test]
    fn deterministic_regardless_of_label_order() {
        let a = canonical_bytes(&sample_batch(vec![("host", "a"), ("region", "eu")]));
        let b = canonical_bytes(&sample_batch(vec![("region", "eu"), ("host", "a")]));
        assert_eq!(a, b);
    }
}
