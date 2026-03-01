use std::collections::HashMap;

use sentinel_common::canonicalize::canonical_bytes;
use sentinel_common::crypto::sign_data;
use sentinel_common::proto::metric::Value;
use sentinel_common::proto::{Batch, Metric};

use sentinel_workers::aggregator::AggregatorStore;
use sentinel_workers::alert::{Condition, Evaluator, Rule, Severity};
use sentinel_workers::dedup::BatchDedup;
use sentinel_workers::transform::transform_batch;
use sentinel_workers::verify::{verify_batch, SecretProvider, VerifyResult};

fn sample_batch(agent_id: &str, batch_id: &str, metrics: Vec<Metric>) -> Batch {
    Batch {
        agent_id: agent_id.into(),
        batch_id: batch_id.into(),
        seq_start: 0,
        seq_end: metrics.len() as u64,
        created_at_ms: 1_700_000_000_000,
        metrics,
        meta: Default::default(),
    }
}

fn cpu_metric(core: usize, value: f64) -> Metric {
    let mut labels = HashMap::new();
    labels.insert("core".into(), core.to_string());
    Metric {
        name: format!("cpu.core.{core}.usage"),
        labels,
        rtype: 1,
        value: Some(Value::ValueDouble(value)),
        timestamp_ms: 1_700_000_000_000,
    }
}

struct FakeSecretProvider {
    secrets: HashMap<String, Vec<u8>>,
}

#[tonic::async_trait]
impl SecretProvider for FakeSecretProvider {
    async fn get_secret(&self, agent_id: &str) -> Option<Vec<u8>> {
        self.secrets.get(agent_id).cloned()
    }
}

#[tokio::test]
async fn full_pipeline_verify_dedup_transform_alert() {
    let secret = b"agent-secret".to_vec();

    let metrics = vec![cpu_metric(0, 95.0), cpu_metric(1, 45.0)];
    let batch = sample_batch("agent-pipeline", "batch-pipe-001", metrics);

    let canonical = canonical_bytes(&batch);
    let signature = sign_data(&secret, &canonical);

    let mut secrets = HashMap::new();
    secrets.insert("agent-pipeline".into(), secret);
    let provider = FakeSecretProvider { secrets };

    let result = verify_batch(&provider, &batch, Some(&signature)).await;
    assert!(matches!(result, VerifyResult::Valid));

    let dedup = BatchDedup::new();
    assert!(!dedup.is_duplicate(&batch.batch_id));
    dedup.mark_processed(batch.batch_id.clone());
    assert!(dedup.is_duplicate(&batch.batch_id));

    let rows = transform_batch(&batch);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].agent_id, "agent-pipeline");
    assert_eq!(rows[0].name, "cpu.core.0.usage");
    assert_eq!(rows[0].value, Some(95.0));

    let aggregator = AggregatorStore::new(60_000);
    for row in &rows {
        if let Some(v) = row.value {
            aggregator.ingest(&row.agent_id, &row.name, row.time_ms, v);
        }
    }

    let rule = Rule {
        id: "rule-cpu-high".into(),
        name: "High CPU".into(),
        agent_pattern: "*".into(),
        metric_name: "cpu.core.0.usage".into(),
        condition: Condition::GreaterThan,
        threshold: 90.0,
        for_duration_ms: 0,
        severity: Severity::Critical,
        annotations: HashMap::new(),
    };

    let evaluator = Evaluator::new(vec![rule]);
    let events = evaluator.evaluate("agent-pipeline", &aggregator, 1_700_000_000_000);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].rule_name, "High CPU");
    assert_eq!(events[0].agent_id, "agent-pipeline");
    assert!(events[0].value >= 95.0);
}

#[tokio::test]
async fn pipeline_bad_signature_rejected() {
    let secret = b"real-secret".to_vec();

    let metrics = vec![cpu_metric(0, 50.0)];
    let batch = sample_batch("agent-bad-sig", "batch-bad-001", metrics);

    let canonical = canonical_bytes(&batch);
    let signature = sign_data(b"wrong-secret", &canonical);

    let mut secrets = HashMap::new();
    secrets.insert("agent-bad-sig".into(), secret);
    let provider = FakeSecretProvider { secrets };

    let result = verify_batch(&provider, &batch, Some(&signature)).await;
    assert!(matches!(result, VerifyResult::Invalid));
}

#[tokio::test]
async fn pipeline_dedup_prevents_reprocessing() {
    let dedup = BatchDedup::new();

    let ids = ["b-1", "b-2", "b-3", "b-1", "b-2"];
    let mut accepted = 0;
    let mut rejected = 0;

    for id in &ids {
        if dedup.is_duplicate(id) {
            rejected += 1;
        } else {
            dedup.mark_processed(id.to_string());
            accepted += 1;
        }
    }

    assert_eq!(accepted, 3);
    assert_eq!(rejected, 2);
}

#[tokio::test]
async fn pipeline_alert_respects_for_duration() {
    let rule = Rule {
        id: "rule-delayed".into(),
        name: "Delayed Alert".into(),
        agent_pattern: "*".into(),
        metric_name: "cpu.core.0.usage".into(),
        condition: Condition::GreaterThan,
        threshold: 80.0,
        for_duration_ms: 60_000,
        severity: Severity::Warning,
        annotations: HashMap::new(),
    };

    let metrics = vec![cpu_metric(0, 90.0)];
    let batch = sample_batch("agent-delay", "batch-d-1", metrics);
    let rows = transform_batch(&batch);

    let aggregator = AggregatorStore::new(60_000);
    for row in &rows {
        if let Some(v) = row.value {
            aggregator.ingest(&row.agent_id, &row.name, row.time_ms, v);
        }
    }

    let evaluator = Evaluator::new(vec![rule]);
    let events = evaluator.evaluate("agent-delay", &aggregator, 1_700_000_000_000);
    assert!(events.is_empty(), "should be pending, not firing yet");
}

#[tokio::test]
async fn pipeline_no_alert_below_threshold() {
    let rule = Rule {
        id: "rule-safe".into(),
        name: "Safe Rule".into(),
        agent_pattern: "*".into(),
        metric_name: "cpu.core.0.usage".into(),
        condition: Condition::GreaterThan,
        threshold: 90.0,
        for_duration_ms: 0,
        severity: Severity::Info,
        annotations: HashMap::new(),
    };

    let metrics = vec![cpu_metric(0, 50.0)];
    let batch = sample_batch("agent-safe", "batch-s-1", metrics);
    let rows = transform_batch(&batch);

    let aggregator = AggregatorStore::new(60_000);
    for row in &rows {
        if let Some(v) = row.value {
            aggregator.ingest(&row.agent_id, &row.name, row.time_ms, v);
        }
    }

    let evaluator = Evaluator::new(vec![rule]);
    let events = evaluator.evaluate("agent-safe", &aggregator, 1_700_000_000_000);
    assert!(events.is_empty());
}
