use sqlx::PgPool;

use super::event::AlertEvent;

pub struct AlertStore {
    pool: PgPool,
}

impl AlertStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn persist(&self, event: &AlertEvent) -> Result<(), sqlx::Error> {
        let annotations_json = serde_json::to_value(&event.annotations).unwrap_or_default();
        let status_str = match event.status {
            super::event::AlertStatus::Firing => "firing",
            super::event::AlertStatus::Resolved => "resolved",
        };
        let severity_str = match event.severity {
            super::rule::Severity::Info => "info",
            super::rule::Severity::Warning => "warning",
            super::rule::Severity::Critical => "critical",
        };

        sqlx::query(
            r#"INSERT INTO alerts
               (id, fingerprint, rule_id, rule_name, agent_id, metric_name,
                severity, status, value, threshold, fired_at, resolved_at, annotations)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                       to_timestamp($11::double precision / 1000),
                       CASE WHEN $12::bigint IS NOT NULL
                            THEN to_timestamp($12::double precision / 1000)
                            ELSE NULL END,
                       $13)"#,
        )
        .bind(&event.id)
        .bind(&event.fingerprint)
        .bind(&event.rule_id)
        .bind(&event.rule_name)
        .bind(&event.agent_id)
        .bind(&event.metric_name)
        .bind(severity_str)
        .bind(status_str)
        .bind(event.value)
        .bind(event.threshold)
        .bind(event.fired_at_ms)
        .bind(event.resolved_at_ms)
        .bind(&annotations_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
