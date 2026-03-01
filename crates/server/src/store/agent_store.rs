use super::agent_record::AgentRecord;
use dashmap::DashMap;
use std::sync::Arc;

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
