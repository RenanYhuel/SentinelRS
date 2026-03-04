use sqlx::PgPool;

pub struct NotifierConfigRow {
    pub id: String,
    pub name: String,
    pub ntype: String,
    pub config: serde_json::Value,
}

pub struct NotifierConfigLoader {
    pool: PgPool,
}

impl NotifierConfigLoader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    pub async fn load_by_ids(&self, ids: &[String]) -> Result<Vec<NotifierConfigRow>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, DbRow>(
            "SELECT id, name, ntype, config
             FROM notifier_configs
             WHERE id = ANY($1) AND enabled = TRUE",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| NotifierConfigRow {
                id: r.id,
                name: r.name,
                ntype: r.ntype,
                config: r.config,
            })
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct DbRow {
    id: String,
    name: String,
    ntype: String,
    config: serde_json::Value,
}
