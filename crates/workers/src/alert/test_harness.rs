use crate::aggregator::AggregatorStore;
use crate::alert::event::{AlertEvent, AlertStatus};
use crate::alert::evaluator::Evaluator;
use crate::alert::rule::Rule;

pub struct MetricSample {
    pub agent_id: String,
    pub metric_name: String,
    pub timestamp_ms: i64,
    pub value: f64,
}

pub struct HarnessResult {
    pub events: Vec<AlertEvent>,
    pub firing_count: usize,
    pub resolved_count: usize,
}

pub fn run_harness(rules: Vec<Rule>, samples: Vec<MetricSample>) -> HarnessResult {
    let aggregator = AggregatorStore::new(60_000);
    let evaluator = Evaluator::new(rules);
    let mut all_events = Vec::new();

    let mut agents: Vec<String> = samples.iter().map(|s| s.agent_id.clone()).collect();
    agents.sort();
    agents.dedup();

    for sample in &samples {
        aggregator.ingest(
            &sample.agent_id,
            &sample.metric_name,
            sample.timestamp_ms,
            sample.value,
        );

        for agent in &agents {
            let events = evaluator.evaluate(agent, &aggregator, sample.timestamp_ms);
            all_events.extend(events);
        }
    }

    let firing_count = all_events
        .iter()
        .filter(|e| e.status == AlertStatus::Firing)
        .count();
    let resolved_count = all_events
        .iter()
        .filter(|e| e.status == AlertStatus::Resolved)
        .count();

    HarnessResult {
        events: all_events,
        firing_count,
        resolved_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::rule::{Condition, Severity};
    use std::collections::HashMap;

    fn high_cpu_rule() -> Rule {
        Rule {
            id: "r-cpu".into(),
            name: "High CPU".into(),
            agent_pattern: "*".into(),
            metric_name: "cpu".into(),
            condition: Condition::GreaterThan,
            threshold: 80.0,
            for_duration_ms: 0,
            severity: Severity::Warning,
            annotations: HashMap::new(),
        }
    }

    #[test]
    fn single_breach_fires_once() {
        let samples = vec![
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 1000, value: 90.0 },
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 2000, value: 95.0 },
        ];
        let result = run_harness(vec![high_cpu_rule()], samples);
        assert_eq!(result.firing_count, 1);
        assert_eq!(result.resolved_count, 0);
    }

    #[test]
    fn breach_then_recovery() {
        let samples = vec![
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 1000, value: 90.0 },
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 2000, value: 50.0 },
        ];
        let result = run_harness(vec![high_cpu_rule()], samples);
        assert_eq!(result.firing_count, 1);
        assert_eq!(result.resolved_count, 1);
    }

    #[test]
    fn no_breach_no_events() {
        let samples = vec![
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 1000, value: 50.0 },
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 2000, value: 60.0 },
        ];
        let result = run_harness(vec![high_cpu_rule()], samples);
        assert_eq!(result.firing_count, 0);
        assert_eq!(result.resolved_count, 0);
    }

    #[test]
    fn with_duration_delays_firing() {
        let mut rule = high_cpu_rule();
        rule.for_duration_ms = 5000;

        let samples = vec![
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 1000, value: 90.0 },
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 3000, value: 90.0 },
        ];
        let result = run_harness(vec![rule.clone()], samples);
        assert_eq!(result.firing_count, 0);

        let samples_long = vec![
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 1000, value: 90.0 },
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 7000, value: 90.0 },
        ];
        let result2 = run_harness(vec![rule], samples_long);
        assert_eq!(result2.firing_count, 1);
    }

    #[test]
    fn multi_agent_independent() {
        let samples = vec![
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 1000, value: 90.0 },
            MetricSample { agent_id: "a2".into(), metric_name: "cpu".into(), timestamp_ms: 1000, value: 90.0 },
        ];
        let result = run_harness(vec![high_cpu_rule()], samples);
        assert_eq!(result.firing_count, 2);
    }

    #[test]
    fn multiple_rules_evaluated() {
        let mem_rule = Rule {
            id: "r-mem".into(),
            name: "High Memory".into(),
            agent_pattern: "*".into(),
            metric_name: "memory".into(),
            condition: Condition::GreaterThan,
            threshold: 90.0,
            for_duration_ms: 0,
            severity: Severity::Critical,
            annotations: HashMap::new(),
        };

        let samples = vec![
            MetricSample { agent_id: "a1".into(), metric_name: "cpu".into(), timestamp_ms: 1000, value: 90.0 },
            MetricSample { agent_id: "a1".into(), metric_name: "memory".into(), timestamp_ms: 1000, value: 95.0 },
        ];
        let result = run_harness(vec![high_cpu_rule(), mem_rule], samples);
        assert_eq!(result.firing_count, 2);
    }
}
