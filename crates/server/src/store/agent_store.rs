use super::agent_record::AgentRecord;
use super::deprecated_key::DeprecatedKey;
use dashmap::DashMap;
use std::sync::Arc;

use sentinel_common::crypto::generate_secret;

#[derive(Clone)]
pub struct AgentStore {
    agents: Arc<DashMap<String, AgentRecord>>,
    by_hw_id: Arc<DashMap<String, String>>,
}

impl AgentStore {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(DashMap::new()),
            by_hw_id: Arc::new(DashMap::new()),
        }
    }

    pub fn insert(&self, record: AgentRecord) {
        self.by_hw_id
            .insert(record.hw_id.clone(), record.agent_id.clone());
        self.agents.insert(record.agent_id.clone(), record);
    }

    pub fn get(&self, agent_id: &str) -> Option<AgentRecord> {
        self.agents.get(agent_id).map(|r| r.clone())
    }

    pub fn find_by_hw_id(&self, hw_id: &str) -> Option<AgentRecord> {
        let agent_id = self.by_hw_id.get(hw_id)?;
        self.get(&agent_id)
    }

    pub fn list(&self) -> Vec<AgentRecord> {
        self.agents.iter().map(|r| r.value().clone()).collect()
    }

    pub fn count(&self) -> usize {
        self.agents.len()
    }

    pub fn rotate_key(&self, agent_id: &str) -> Option<(String, Vec<u8>)> {
        let mut entry = self.agents.get_mut(agent_id)?;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let old = DeprecatedKey {
            key_id: entry.key_id.clone(),
            secret: entry.secret.clone(),
            deprecated_at_ms: now_ms,
        };
        entry.deprecated_keys.push(old);

        let new_secret = generate_secret();
        let new_key_id = format!("key-{}", uuid::Uuid::new_v4());
        entry.secret = new_secret.clone();
        entry.key_id = new_key_id.clone();
        Some((new_key_id, new_secret))
    }

    pub fn find_key_secret(
        &self,
        agent_id: &str,
        key_id: Option<&str>,
        grace_period_ms: i64,
    ) -> Option<Vec<u8>> {
        let record = self.agents.get(agent_id)?;

        match key_id {
            Some(kid) if kid == record.key_id => Some(record.secret.clone()),
            Some(kid) => {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                record
                    .deprecated_keys
                    .iter()
                    .find(|dk| dk.key_id == kid && (now_ms - dk.deprecated_at_ms) < grace_period_ms)
                    .map(|dk| dk.secret.clone())
            }
            None => Some(record.secret.clone()),
        }
    }

    pub fn purge_expired_keys(&self, grace_period_ms: i64) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        for mut entry in self.agents.iter_mut() {
            entry
                .deprecated_keys
                .retain(|dk| (now_ms - dk.deprecated_at_ms) < grace_period_ms);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record() -> AgentRecord {
        AgentRecord {
            agent_id: "agent-1".into(),
            hw_id: "hw-abc".into(),
            secret: b"secret".to_vec(),
            key_id: "key-1".into(),
            agent_version: "0.1.0".into(),
            registered_at_ms: 1000,
            deprecated_keys: Vec::new(),
        }
    }

    #[test]
    fn insert_and_get() {
        let store = AgentStore::new();
        store.insert(sample_record());
        let r = store.get("agent-1").unwrap();
        assert_eq!(r.hw_id, "hw-abc");
    }

    #[test]
    fn find_by_hw_id() {
        let store = AgentStore::new();
        store.insert(sample_record());
        let r = store.find_by_hw_id("hw-abc").unwrap();
        assert_eq!(r.agent_id, "agent-1");
    }

    #[test]
    fn list_returns_all() {
        let store = AgentStore::new();
        store.insert(sample_record());
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn missing_returns_none() {
        let store = AgentStore::new();
        assert!(store.get("nope").is_none());
        assert!(store.find_by_hw_id("nope").is_none());
    }
}
