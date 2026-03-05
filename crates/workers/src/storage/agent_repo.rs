use sqlx::PgPool;

pub struct AgentRepo {
    pool: PgPool,
}

impl AgentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn touch_last_seen(&self, agent_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE agents SET last_seen = NOW(), status = 'online' WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
