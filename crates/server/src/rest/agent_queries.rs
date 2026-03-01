use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize, sqlx::FromRow)]
pub struct AgentSummaryRow {
    pub agent_id: String,
    pub hw_id: String,
    pub agent_version: String,
    pub registered_at_ms: i64,
    pub last_seen: Option<DateTime<Utc>>,
}

pub async fn fetch_all(pool: &PgPool) -> Result<Vec<AgentSummaryRow>, sqlx::Error> {
    sqlx::query_as::<_, AgentSummaryRow>(
        "SELECT agent_id, hw_id, agent_version, registered_at_ms, last_seen FROM agents ORDER BY agent_id",
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_one(
    pool: &PgPool,
    agent_id: &str,
) -> Result<Option<AgentSummaryRow>, sqlx::Error> {
    sqlx::query_as::<_, AgentSummaryRow>(
        "SELECT agent_id, hw_id, agent_version, registered_at_ms, last_seen FROM agents WHERE agent_id = $1",
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await
}
