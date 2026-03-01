use crate::security::HmacSigner;
use sentinel_common::proto::{metric::Value, Batch};

pub struct HttpFallbackClient {
    base_url: String,
    agent_id: String,
    signer: HmacSigner,
    key_id: String,
    http: reqwest::Client,
}

#[derive(Debug)]
pub enum HttpFallbackError {
    Serialize(String),
    Transport(String),
    Rejected(u16),
}

impl std::fmt::Display for HttpFallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(e) => write!(f, "serialize: {e}"),
            Self::Transport(e) => write!(f, "transport: {e}"),
            Self::Rejected(code) => write!(f, "rejected with status {code}"),
        }
    }
}

impl std::error::Error for HttpFallbackError {}

impl HttpFallbackClient {
    pub fn new(base_url: &str, agent_id: String, secret: &[u8], key_id: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            agent_id,
            signer: HmacSigner::new(secret),
            key_id,
            http: reqwest::Client::new(),
        }
    }

    pub async fn push_metrics(&self, batch: &Batch) -> Result<(), HttpFallbackError> {
        let canonical = sentinel_common::canonicalize::canonical_bytes(batch);
        let signature = self.signer.sign_base64(&canonical);

        let body = encode_batch_json(batch)?;

        let resp = self
            .http
            .post(format!("{}/v1/agent/metrics", self.base_url))
            .header("Content-Type", "application/json")
            .header("X-Agent-Id", &self.agent_id)
            .header("X-Signature", &signature)
            .header("X-Key-Id", &self.key_id)
            .body(body)
            .send()
            .await
            .map_err(|e| HttpFallbackError::Transport(e.to_string()))?;

        let status = resp.status().as_u16();
        if status >= 200 && status < 300 {
            Ok(())
        } else {
            Err(HttpFallbackError::Rejected(status))
        }
    }
}

fn encode_batch_json(batch: &Batch) -> Result<Vec<u8>, HttpFallbackError> {
    serde_json::to_vec(&BatchPayload::from(batch))
        .map_err(|e| HttpFallbackError::Serialize(e.to_string()))
}

#[derive(serde::Serialize)]
struct BatchPayload {
    batch_id: String,
    agent_id: String,
    seq_start: u64,
    seq_end: u64,
    metrics: Vec<MetricPayload>,
}

#[derive(serde::Serialize)]
struct MetricPayload {
    name: String,
    value: f64,
    labels: std::collections::HashMap<String, String>,
    timestamp: u64,
}

impl From<&Batch> for BatchPayload {
    fn from(b: &Batch) -> Self {
        Self {
            batch_id: b.batch_id.clone(),
            agent_id: b.agent_id.clone(),
            seq_start: b.seq_start,
            seq_end: b.seq_end,
            metrics: b
                .metrics
                .iter()
                .map(|m| {
                    let value = match &m.value {
                        Some(Value::ValueDouble(v)) => *v,
                        Some(Value::ValueInt(v)) => *v as f64,
                        _ => 0.0,
                    };
                    MetricPayload {
                        name: m.name.clone(),
                        value,
                        labels: m.labels.clone(),
                        timestamp: m.timestamp_ms as u64,
                    }
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_common::proto::{metric::Value, Metric};

    fn sample_batch() -> Batch {
        Batch {
            batch_id: "batch-1".into(),
            agent_id: "agent-1".into(),
            seq_start: 0,
            seq_end: 1,
            metrics: vec![Metric {
                name: "cpu.usage".into(),
                value: Some(Value::ValueDouble(55.5)),
                labels: [("host".into(), "srv1".into())].into(),
                timestamp_ms: 1000,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn batch_payload_serializes() {
        let batch = sample_batch();
        let json = encode_batch_json(&batch).unwrap();
        let text = String::from_utf8(json).unwrap();
        assert!(text.contains("batch-1"));
        assert!(text.contains("cpu.usage"));
        assert!(text.contains("55.5"));
    }

    #[test]
    fn error_display() {
        let e = HttpFallbackError::Rejected(503);
        assert!(e.to_string().contains("503"));
    }
}
