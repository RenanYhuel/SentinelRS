use sqlx::PgPool;

pub struct NotifierConfigRecord {
    pub id: String,
    pub name: String,
    pub ntype: String,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at_ms: i64,
}

pub struct NotifierRepo {
    pool: PgPool,
}

impl NotifierRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, r: &NotifierConfigRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO notifier_configs (id, name, ntype, config, enabled, created_at)
               VALUES ($1, $2, $3, $4, $5, to_timestamp($6::double precision / 1000))
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(&r.id)
        .bind(&r.name)
        .bind(&r.ntype)
        .bind(&r.config)
        .bind(r.enabled)
        .bind(r.created_at_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_all(&self) -> Result<Vec<NotifierConfigRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, NotifierRow>(
            "SELECT id, name, ntype, config, enabled,
                    EXTRACT(EPOCH FROM created_at)::bigint * 1000 AS created_at_ms
             FROM notifier_configs ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get(&self, id: &str) -> Result<Option<NotifierConfigRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, NotifierRow>(
            "SELECT id, name, ntype, config, enabled,
                    EXTRACT(EPOCH FROM created_at)::bigint * 1000 AS created_at_ms
             FROM notifier_configs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn get_by_ids(
        &self,
        ids: &[String],
    ) -> Result<Vec<NotifierConfigRecord>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, NotifierRow>(
            "SELECT id, name, ntype, config, enabled,
                    EXTRACT(EPOCH FROM created_at)::bigint * 1000 AS created_at_ms
             FROM notifier_configs WHERE id = ANY($1) AND enabled = TRUE",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM notifier_configs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[derive(sqlx::FromRow)]
struct NotifierRow {
    id: String,
    name: String,
    ntype: String,
    config: serde_json::Value,
    enabled: bool,
    created_at_ms: i64,
}

impl From<NotifierRow> for NotifierConfigRecord {
    fn from(row: NotifierRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            ntype: row.ntype,
            config: row.config,
            enabled: row.enabled,
            created_at_ms: row.created_at_ms,
        }
    }
}
