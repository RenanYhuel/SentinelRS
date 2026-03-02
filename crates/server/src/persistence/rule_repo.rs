use sqlx::PgPool;

use crate::store::rule_record::RuleRecord;

pub struct RuleRepo {
    pool: PgPool,
}

impl RuleRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, r: &RuleRecord) -> Result<(), sqlx::Error> {
        let annotations = serde_json::to_value(&r.annotations).unwrap_or_default();
        sqlx::query(
            r#"INSERT INTO alert_rules
               (id, name, agent_pattern, metric_name, condition, threshold,
                for_duration_ms, severity, annotations, enabled, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                       to_timestamp($11::double precision / 1000),
                       to_timestamp($12::double precision / 1000))
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(&r.id)
        .bind(&r.name)
        .bind(&r.agent_pattern)
        .bind(&r.metric_name)
        .bind(&r.condition)
        .bind(r.threshold)
        .bind(r.for_duration_ms)
        .bind(&r.severity)
        .bind(&annotations)
        .bind(r.enabled)
        .bind(r.created_at_ms)
        .bind(r.updated_at_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update(&self, r: &RuleRecord) -> Result<(), sqlx::Error> {
        let annotations = serde_json::to_value(&r.annotations).unwrap_or_default();
        sqlx::query(
            r#"UPDATE alert_rules SET
                 name = $2, agent_pattern = $3, metric_name = $4,
                 condition = $5, threshold = $6, for_duration_ms = $7,
                 severity = $8, annotations = $9, enabled = $10,
                 updated_at = to_timestamp($11::double precision / 1000)
               WHERE id = $1"#,
        )
        .bind(&r.id)
        .bind(&r.name)
        .bind(&r.agent_pattern)
        .bind(&r.metric_name)
        .bind(&r.condition)
        .bind(r.threshold)
        .bind(r.for_duration_ms)
        .bind(&r.severity)
        .bind(&annotations)
        .bind(r.enabled)
        .bind(r.updated_at_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM alert_rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn load_all(&self, store: &crate::store::RuleStore) -> Result<usize, sqlx::Error> {
        let rows = sqlx::query_as::<_, RuleRow>(
            "SELECT id, name, agent_pattern, metric_name, condition, threshold,
                    for_duration_ms, severity, annotations, enabled,
                    EXTRACT(EPOCH FROM created_at)::bigint * 1000 AS created_at_ms,
                    EXTRACT(EPOCH FROM updated_at)::bigint * 1000 AS updated_at_ms
             FROM alert_rules",
        )
        .fetch_all(&self.pool)
        .await?;

        let count = rows.len();
        for row in rows {
            let annotations: std::collections::HashMap<String, String> =
                serde_json::from_value(row.annotations).unwrap_or_default();
            store.insert(RuleRecord {
                id: row.id,
                name: row.name,
                agent_pattern: row.agent_pattern,
                metric_name: row.metric_name,
                condition: row.condition,
                threshold: row.threshold,
                for_duration_ms: row.for_duration_ms,
                severity: row.severity,
                annotations,
                enabled: row.enabled,
                created_at_ms: row.created_at_ms,
                updated_at_ms: row.updated_at_ms,
            });
        }
        Ok(count)
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
    enabled: bool,
    created_at_ms: i64,
    updated_at_ms: i64,
}
