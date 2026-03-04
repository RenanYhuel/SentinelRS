use sqlx::PgPool;
use std::collections::HashMap;

use crate::alert::{Condition, Rule, Severity};

pub struct RuleLoader {
    pool: PgPool,
}

impl RuleLoader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn load_enabled(&self) -> Result<Vec<Rule>, sqlx::Error> {
        let rows = sqlx::query_as::<_, RuleRow>(
            "SELECT id, name, agent_pattern, metric_name, condition, threshold,
                    for_duration_ms, severity, annotations, notifier_ids
             FROM alert_rules WHERE enabled = TRUE",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().filter_map(|r| r.into_rule()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct RuleRow {
    id: String,
    name: String,
    agent_pattern: String,
    metric_name: String,
    condition: String,
    threshold: f64,
    for_duration_ms: i64,
    severity: String,
    annotations: serde_json::Value,
    notifier_ids: serde_json::Value,
}

impl RuleRow {
    fn into_rule(self) -> Option<Rule> {
        let condition = match self.condition.as_str() {
            "GreaterThan" => Condition::GreaterThan,
            "LessThan" => Condition::LessThan,
            "GreaterOrEqual" => Condition::GreaterOrEqual,
            "LessOrEqual" => Condition::LessOrEqual,
            "Equal" => Condition::Equal,
            _ => return None,
        };
        let severity = match self.severity.as_str() {
            "info" => Severity::Info,
            "warning" => Severity::Warning,
            "critical" => Severity::Critical,
            _ => Severity::Warning,
        };
        let annotations: HashMap<String, String> =
            serde_json::from_value(self.annotations).unwrap_or_default();
        let notifier_ids: Vec<String> =
            serde_json::from_value(self.notifier_ids).unwrap_or_default();

        Some(Rule {
            id: self.id,
            name: self.name,
            agent_pattern: self.agent_pattern,
            metric_name: self.metric_name,
            condition,
            threshold: self.threshold,
            for_duration_ms: self.for_duration_ms,
            severity,
            annotations,
            notifier_ids,
        })
    }
}
