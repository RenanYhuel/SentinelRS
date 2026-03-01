use serde::{Deserialize, Serialize};

use super::rule::Severity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub id: String,
    pub fingerprint: String,
    pub rule_id: String,
    pub rule_name: String,
    pub agent_id: String,
    pub metric_name: String,
    pub severity: Severity,
    pub status: AlertStatus,
    pub value: f64,
    pub threshold: f64,
    pub fired_at_ms: i64,
    pub resolved_at_ms: Option<i64>,
    pub annotations: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AlertStatus {
    Firing,
    Resolved,
}

impl AlertEvent {
    pub fn severity_str(&self) -> &str {
        match self.severity {
            super::rule::Severity::Info => "INFO",
            super::rule::Severity::Warning => "WARN",
            super::rule::Severity::Critical => "CRIT",
        }
    }

    pub fn status_str(&self) -> &str {
        match self.status {
            AlertStatus::Firing => "firing",
            AlertStatus::Resolved => "resolved",
        }
    }
}
