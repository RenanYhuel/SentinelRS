use dashmap::DashMap;
use std::sync::Arc;

use super::event::{AlertEvent, AlertStatus};
use super::fingerprint::fingerprint_string;
use super::rule::Rule;
use super::state::RuleState;
use crate::aggregator::AggregatorStore;

pub struct Evaluator {
    rules: Vec<Rule>,
    states: Arc<DashMap<String, RuleState>>,
}

impl Evaluator {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self {
            rules,
            states: Arc::new(DashMap::new()),
        }
    }

    pub fn set_rules(&mut self, rules: Vec<Rule>) {
        self.rules = rules;
        self.states.clear();
    }

    pub fn evaluate(
        &self,
        agent_id: &str,
        aggregator: &AggregatorStore,
        now_ms: i64,
    ) -> Vec<AlertEvent> {
        let mut events = Vec::new();

        for rule in &self.rules {
            if !agent_matches(&rule.agent_pattern, agent_id) {
                continue;
            }

            let fp = fingerprint_string(&rule.id, agent_id, &rule.metric_name);

            let value = match aggregator.avg(agent_id, &rule.metric_name) {
                Some(v) => v,
                None => continue,
            };

            let condition_met = rule.condition.evaluate(value, rule.threshold);

            let current = self.states.get(&fp).map(|s| *s).unwrap_or(RuleState::Ok);
            let next = current.transition(condition_met, now_ms, rule.for_duration_ms);
            self.states.insert(fp.clone(), next);

            if next.is_firing() && !current.is_firing() {
                events.push(AlertEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    fingerprint: fp.clone(),
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    agent_id: agent_id.to_string(),
                    metric_name: rule.metric_name.clone(),
                    severity: rule.severity,
                    status: AlertStatus::Firing,
                    value,
                    threshold: rule.threshold,
                    fired_at_ms: now_ms,
                    resolved_at_ms: None,
                    annotations: rule.annotations.clone(),
                });
            }

            if next.just_resolved() {
                events.push(AlertEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    fingerprint: fp,
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    agent_id: agent_id.to_string(),
                    metric_name: rule.metric_name.clone(),
                    severity: rule.severity,
                    status: AlertStatus::Resolved,
                    value,
                    threshold: rule.threshold,
                    fired_at_ms: now_ms,
                    resolved_at_ms: Some(now_ms),
                    annotations: rule.annotations.clone(),
                });
            }
        }

        events
    }
}

fn agent_matches(pattern: &str, agent_id: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        return agent_id.starts_with(prefix);
    }
    pattern == agent_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::rule::{Condition, Severity};
    use std::collections::HashMap;

    fn cpu_rule() -> Rule {
        Rule {
            id: "r-1".into(),
            name: "high cpu".into(),
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
    fn fires_when_threshold_exceeded() {
        let agg = AggregatorStore::new(10000);
        agg.ingest("agent-1", "cpu", 1000, 90.0);

        let eval = Evaluator::new(vec![cpu_rule()]);
        let events = eval.evaluate("agent-1", &agg, 1000);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, AlertStatus::Firing);
    }

    #[test]
    fn no_event_below_threshold() {
        let agg = AggregatorStore::new(10000);
        agg.ingest("agent-1", "cpu", 1000, 50.0);

        let eval = Evaluator::new(vec![cpu_rule()]);
        let events = eval.evaluate("agent-1", &agg, 1000);
        assert!(events.is_empty());
    }

    #[test]
    fn fires_only_once() {
        let agg = AggregatorStore::new(10000);
        agg.ingest("agent-1", "cpu", 1000, 90.0);

        let eval = Evaluator::new(vec![cpu_rule()]);
        let events1 = eval.evaluate("agent-1", &agg, 1000);
        assert_eq!(events1.len(), 1);

        agg.ingest("agent-1", "cpu", 2000, 95.0);
        let events2 = eval.evaluate("agent-1", &agg, 2000);
        assert!(events2.is_empty());
    }

    #[test]
    fn resolves_when_back_below() {
        let agg = AggregatorStore::new(10000);
        agg.ingest("agent-1", "cpu", 1000, 90.0);

        let eval = Evaluator::new(vec![cpu_rule()]);
        eval.evaluate("agent-1", &agg, 1000);

        agg.ingest("agent-1", "cpu", 2000, 50.0);
        let events = eval.evaluate("agent-1", &agg, 2000);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, AlertStatus::Resolved);
    }

    #[test]
    fn agent_pattern_wildcard_prefix() {
        assert!(agent_matches("agent-*", "agent-1"));
        assert!(!agent_matches("agent-*", "server-1"));
    }

    #[test]
    fn agent_pattern_exact() {
        assert!(agent_matches("agent-1", "agent-1"));
        assert!(!agent_matches("agent-1", "agent-2"));
    }

    #[test]
    fn pending_duration() {
        let mut rule = cpu_rule();
        rule.for_duration_ms = 5000;

        let agg = AggregatorStore::new(10000);
        agg.ingest("a", "cpu", 1000, 90.0);

        let eval = Evaluator::new(vec![rule]);
        let events1 = eval.evaluate("a", &agg, 1000);
        assert!(events1.is_empty());

        agg.ingest("a", "cpu", 6000, 90.0);
        let events2 = eval.evaluate("a", &agg, 6000);
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].status, AlertStatus::Firing);
    }
}
