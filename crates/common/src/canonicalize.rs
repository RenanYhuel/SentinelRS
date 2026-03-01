use crate::proto::Batch;

pub fn canonical_bytes(batch: &Batch) -> Vec<u8> {
    let mut out = Vec::new();
    write_str(&mut out, &batch.agent_id);
    write_str(&mut out, &batch.batch_id);
    out.extend_from_slice(&batch.seq_start.to_le_bytes());
    out.extend_from_slice(&batch.seq_end.to_le_bytes());
    out.extend_from_slice(&batch.created_at_ms.to_le_bytes());

    for m in &batch.metrics {
        write_str(&mut out, &m.name);
        let mut labels: Vec<(&String, &String)> = m.labels.iter().collect();
        labels.sort_by_key(|(k, _)| k.as_str());
        for (k, v) in labels {
            write_str(&mut out, k);
            write_str(&mut out, v);
        }
        out.extend_from_slice(&m.rtype.to_le_bytes());
        if let Some(ref val) = m.value {
            encode_value(&mut out, val);
        }
        out.extend_from_slice(&m.timestamp_ms.to_le_bytes());
    }

    let mut meta: Vec<(&String, &String)> = batch.meta.iter().collect();
    meta.sort_by_key(|(k, _)| k.as_str());
    for (k, v) in meta {
        write_str(&mut out, k);
        write_str(&mut out, v);
    }

    out
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn encode_value(out: &mut Vec<u8>, val: &crate::proto::metric::Value) {
    use crate::proto::metric::Value;
    match val {
        Value::ValueDouble(d) => {
            out.push(0x01);
            out.extend_from_slice(&d.to_le_bytes());
        }
        Value::ValueInt(i) => {
            out.push(0x02);
            out.extend_from_slice(&i.to_le_bytes());
        }
        Value::Histogram(h) => {
            out.push(0x03);
            out.extend_from_slice(&(h.boundaries.len() as u32).to_le_bytes());
            for b in &h.boundaries {
                out.extend_from_slice(&b.to_le_bytes());
            }
            out.extend_from_slice(&(h.counts.len() as u32).to_le_bytes());
            for c in &h.counts {
                out.extend_from_slice(&c.to_le_bytes());
            }
            out.extend_from_slice(&h.sum.to_le_bytes());
            out.extend_from_slice(&h.count.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::Metric;
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
