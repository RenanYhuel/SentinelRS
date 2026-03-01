use dashmap::DashMap;
use std::sync::Arc;

use super::rule_record::RuleRecord;

#[derive(Clone)]
pub struct RuleStore {
    rules: Arc<DashMap<String, RuleRecord>>,
}

impl Default for RuleStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleStore {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
        }
    }

    pub fn insert(&self, record: RuleRecord) {
        self.rules.insert(record.id.clone(), record);
    }

    pub fn get(&self, id: &str) -> Option<RuleRecord> {
        self.rules.get(id).map(|r| r.clone())
    }

    pub fn list(&self) -> Vec<RuleRecord> {
        self.rules.iter().map(|r| r.value().clone()).collect()
    }

    pub fn list_enabled(&self) -> Vec<RuleRecord> {
        self.rules
            .iter()
            .filter(|r| r.value().enabled)
            .map(|r| r.value().clone())
            .collect()
    }

    pub fn update(&self, record: RuleRecord) -> bool {
        if self.rules.contains_key(&record.id) {
            self.rules.insert(record.id.clone(), record);
            true
        } else {
            false
        }
    }

    pub fn delete(&self, id: &str) -> bool {
        self.rules.remove(id).is_some()
    }

    pub fn count(&self) -> usize {
        self.rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_rule() -> RuleRecord {
        RuleRecord {
            id: "rule-1".into(),
            name: "High CPU".into(),
            agent_pattern: "*".into(),
            metric_name: "cpu".into(),
            condition: "GreaterThan".into(),
            threshold: 80.0,
            for_duration_ms: 0,
            severity: "warning".into(),
            annotations: HashMap::new(),
            enabled: true,
            created_at_ms: 1000,
            updated_at_ms: 1000,
        }
    }

    #[test]
    fn insert_and_get() {
        let store = RuleStore::new();
        store.insert(sample_rule());
        let r = store.get("rule-1").unwrap();
        assert_eq!(r.name, "High CPU");
    }

    #[test]
    fn list_returns_all() {
        let store = RuleStore::new();
        store.insert(sample_rule());
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn list_enabled_filters() {
        let store = RuleStore::new();
        store.insert(sample_rule());
        let mut disabled = sample_rule();
        disabled.id = "rule-2".into();
        disabled.enabled = false;
        store.insert(disabled);
        assert_eq!(store.list_enabled().len(), 1);
    }

    #[test]
    fn update_existing() {
        let store = RuleStore::new();
        store.insert(sample_rule());
        let mut updated = sample_rule();
        updated.threshold = 90.0;
        assert!(store.update(updated));
        assert_eq!(store.get("rule-1").unwrap().threshold, 90.0);
    }

    #[test]
    fn update_missing_returns_false() {
        let store = RuleStore::new();
        assert!(!store.update(sample_rule()));
    }

    #[test]
    fn delete_existing() {
        let store = RuleStore::new();
        store.insert(sample_rule());
        assert!(store.delete("rule-1"));
        assert!(store.get("rule-1").is_none());
    }

    #[test]
    fn delete_missing_returns_false() {
        let store = RuleStore::new();
        assert!(!store.delete("nope"));
    }
}
