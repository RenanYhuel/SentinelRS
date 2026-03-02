use dashmap::DashMap;
use std::sync::Arc;

use super::session::{Session, SessionSnapshot};

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

    pub fn record_heartbeat(
        &self,
        agent_id: &str,
        latency_ms: i64,
        stats: super::session::LiveSystemStats,
    ) {
        if let Some(mut session) = self.sessions.get_mut(agent_id) {
            session.touch();
            session.record_latency(latency_ms);
            session.update_system_stats(stats);
        }
    }

    pub fn connected_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn connected_agent_ids(&self) -> Vec<String> {
        self.sessions.iter().map(|r| r.key().clone()).collect()
    }

    pub fn snapshot(&self, agent_id: &str) -> Option<SessionSnapshot> {
        self.sessions.get(agent_id).map(|s| s.snapshot())
    }

    pub fn all_snapshots(&self) -> Vec<SessionSnapshot> {
        self.sessions.iter().map(|r| r.value().snapshot()).collect()
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

    pub fn find_stale(&self, timeout_ms: i64) -> Vec<(String, i64)> {
        self.sessions
            .iter()
            .filter_map(|r| {
                if r.value().is_stale(timeout_ms) {
                    Some((r.key().clone(), r.value().ms_since_last_ping()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn cluster_stats(&self) -> ClusterStats {
        let snapshots = self.all_snapshots();
        let total = snapshots.len();

        if total == 0 {
            return ClusterStats::default();
        }

        let total_cpu: f64 = snapshots.iter().map(|s| s.system_stats.cpu_percent).sum();
        let total_mem: f64 = snapshots.iter().map(|s| s.memory_percent).sum();
        let total_latency: f64 = snapshots.iter().map(|s| s.latency.avg_ms).sum();

        ClusterStats {
            connected_agents: total,
            avg_cpu_percent: total_cpu / total as f64,
            avg_memory_percent: total_mem / total as f64,
            avg_latency_ms: total_latency / total as f64,
            total_heartbeats: snapshots.iter().map(|s| s.heartbeat_count).sum(),
            agents: snapshots,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ClusterStats {
    pub connected_agents: usize,
    pub avg_cpu_percent: f64,
    pub avg_memory_percent: f64,
    pub avg_latency_ms: f64,
    pub total_heartbeats: u64,
    pub agents: Vec<SessionSnapshot>,
}
