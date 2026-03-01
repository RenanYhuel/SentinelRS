use prost::Message;

use sentinel_common::batch_id;
use sentinel_common::proto::Batch;
use sentinel_common::proto::Metric;

pub struct BatchComposer {
    agent_id: String,
    seq_counter: u64,
}

impl BatchComposer {
    pub fn new(agent_id: String, initial_seq: u64) -> Self {
        Self {
            agent_id,
            seq_counter: initial_seq,
        }
    }

    pub fn compose(&mut self, metrics: Vec<Metric>) -> Batch {
        let seq_start = self.seq_counter;
        let seq_end = seq_start + metrics.len() as u64;
        self.seq_counter = seq_end;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        Batch {
            agent_id: self.agent_id.clone(),
            batch_id: batch_id::generate(),
            seq_start,
            seq_end,
            created_at_ms: now_ms,
            metrics,
            meta: Default::default(),
        }
    }

    pub fn current_seq(&self) -> u64 {
        self.seq_counter
    }

    pub fn encode_batch(batch: &Batch) -> Vec<u8> {
        batch.encode_to_vec()
    }

    pub fn decode_batch(bytes: &[u8]) -> Result<Batch, prost::DecodeError> {
        Batch::decode(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_common::proto::metric::Value;

    fn sample_metrics(n: usize) -> Vec<Metric> {
        (0..n)
            .map(|i| Metric {
                name: format!("test.metric.{}", i),
                labels: Default::default(),
                rtype: 1,
                value: Some(Value::ValueDouble(i as f64)),
                timestamp_ms: 1000 + i as i64,
            })
            .collect()
    }

    #[test]
    fn compose_assigns_sequential_ranges() {
        let mut composer = BatchComposer::new("agent-test".into(), 0);
        let b1 = composer.compose(sample_metrics(3));
        assert_eq!(b1.seq_start, 0);
        assert_eq!(b1.seq_end, 3);

        let b2 = composer.compose(sample_metrics(2));
        assert_eq!(b2.seq_start, 3);
        assert_eq!(b2.seq_end, 5);
    }

    #[test]
    fn batch_has_unique_id() {
        let mut composer = BatchComposer::new("agent-test".into(), 0);
        let b1 = composer.compose(sample_metrics(1));
        let b2 = composer.compose(sample_metrics(1));
        assert_ne!(b1.batch_id, b2.batch_id);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let mut composer = BatchComposer::new("agent-test".into(), 0);
        let batch = composer.compose(sample_metrics(5));
        let bytes = BatchComposer::encode_batch(&batch);
        let decoded = BatchComposer::decode_batch(&bytes).unwrap();
        assert_eq!(decoded.agent_id, batch.agent_id);
        assert_eq!(decoded.metrics.len(), 5);
        assert_eq!(decoded.seq_start, batch.seq_start);
        assert_eq!(decoded.seq_end, batch.seq_end);
    }
}
