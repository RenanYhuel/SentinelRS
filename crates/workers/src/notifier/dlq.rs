use sqlx::PgPool;

pub struct DlqWriter {
    pool: PgPool,
}

impl DlqWriter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        alert_id: &str,
        notifier: &str,
        payload: &serde_json::Value,
        error: &str,
        attempts: u32,
    ) -> Result<(), sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            r#"INSERT INTO notifications_dlq
               (id, alert_id, notifier, payload, error, attempts)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(&id)
        .bind(alert_id)
        .bind(notifier)
        .bind(payload)
        .bind(error)
        .bind(attempts as i32)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_by_notifier(
        &self,
        notifier: &str,
        limit: i64,
    ) -> Result<Vec<DlqEntry>, sqlx::Error> {
        let rows = sqlx::query_as::<_, DlqEntry>(
            "SELECT id, alert_id, notifier, error, attempts, created_at \
             FROM notifications_dlq WHERE notifier = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(notifier)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM notifications_dlq WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct DlqEntry {
    pub id: String,
    pub alert_id: String,
    pub notifier: String,
    pub error: String,
    pub attempts: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
