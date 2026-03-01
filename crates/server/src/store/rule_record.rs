use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleRecord {
    pub id: String,
    pub name: String,
    pub agent_pattern: String,
    pub metric_name: String,
    pub condition: String,
    pub threshold: f64,
    pub for_duration_ms: i64,
    pub severity: String,
    pub annotations: HashMap<String, String>,
    pub enabled: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}
