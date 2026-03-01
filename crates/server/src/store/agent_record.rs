use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    pub agent_id: String,
    pub hw_id: String,
    pub secret: Vec<u8>,
    pub key_id: String,
    pub agent_version: String,
    pub registered_at_ms: i64,
}
