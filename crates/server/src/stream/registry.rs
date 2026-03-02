use dashmap::DashMap;
use std::sync::Arc;

use super::session::Session;

#[derive(Clone)]
pub struct SessionRegistry {
    sessions: Arc<DashMap<String, Session>>,
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, session: Session) -> bool {
        let agent_id = session.agent_id.clone();
        if self.sessions.contains_key(&agent_id) {
            return false;
        }
        self.sessions.insert(agent_id, session);
        true
    }

    pub fn replace(&self, session: Session) {
        let agent_id = session.agent_id.clone();
        self.sessions.insert(agent_id, session);
    }

    pub fn unregister(&self, agent_id: &str) -> Option<Session> {
        self.sessions.remove(agent_id).map(|(_, s)| s)
    }

    pub fn contains(&self, agent_id: &str) -> bool {
        self.sessions.contains_key(agent_id)
    }

    pub fn touch(&self, agent_id: &str) {
        if let Some(mut session) = self.sessions.get_mut(agent_id) {
            session.touch();
        }
    }

    pub fn connected_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn connected_agent_ids(&self) -> Vec<String> {
        self.sessions.iter().map(|r| r.key().clone()).collect()
    }

    pub fn evict_stale(&self, timeout_ms: i64) -> Vec<String> {
        let mut evicted = Vec::new();
        self.sessions.retain(|id, session| {
            if session.is_stale(timeout_ms) {
                evicted.push(id.clone());
                false
            } else {
                true
            }
        });
        evicted
    }
}
