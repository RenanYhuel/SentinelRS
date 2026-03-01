use sqlx::PgPool;

pub struct RawWriter {
    pool: PgPool,
}

impl RawWriter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_raw_batch(
        &self,
        agent_id: &str,
        batch_id: &str,
        payload: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO metrics_raw (agent_id, batch_id, payload) VALUES ($1, $2, $3)")
            .bind(agent_id)
            .bind(batch_id)
            .bind(payload)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
