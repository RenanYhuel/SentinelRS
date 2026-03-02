use sqlx::PgPool;

use crate::store::{AgentRecord, AgentStore, DeprecatedKey};

pub struct AgentRepo {
    pool: PgPool,
}

impl AgentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn load_all(&self, store: &AgentStore) -> Result<usize, sqlx::Error> {
        let rows = sqlx::query_as::<_, AgentRow>(
            "SELECT agent_id, hw_id, secret, key_id, agent_version, registered_at_ms, deprecated_keys, last_seen FROM agents",
        )
        .fetch_all(&self.pool)
        .await?;

        let count = rows.len();
        for row in rows {
            let deprecated_keys: Vec<DeprecatedKey> =
                serde_json::from_value(row.deprecated_keys).unwrap_or_default();

            store.insert(AgentRecord {
                agent_id: row.agent_id,
                hw_id: row.hw_id,
                secret: row.secret,
                key_id: row.key_id,
                agent_version: row.agent_version,
                registered_at_ms: row.registered_at_ms,
                deprecated_keys,
                last_seen: row.last_seen,
            });
        }

        Ok(count)
    }

    pub async fn upsert(&self, record: &AgentRecord) -> Result<(), sqlx::Error> {
        let deprecated_json = serde_json::to_value(&record.deprecated_keys)
            .unwrap_or_else(|_| serde_json::Value::Array(vec![]));

        sqlx::query(
            r#"INSERT INTO agents (agent_id, hw_id, secret, key_id, agent_version, registered_at_ms, deprecated_keys, last_seen)
               VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
               ON CONFLICT (agent_id) DO UPDATE SET
                 hw_id = EXCLUDED.hw_id,
                 secret = EXCLUDED.secret,
                 key_id = EXCLUDED.key_id,
                 agent_version = EXCLUDED.agent_version,
                 deprecated_keys = EXCLUDED.deprecated_keys,
                 last_seen = NOW()"#,
        )
        .bind(&record.agent_id)
        .bind(&record.hw_id)
        .bind(&record.secret)
        .bind(&record.key_id)
        .bind(&record.agent_version)
        .bind(record.registered_at_ms)
        .bind(&deprecated_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct AgentRow {
    agent_id: String,
    hw_id: String,
    secret: Vec<u8>,
    key_id: String,
    agent_version: String,
    registered_at_ms: i64,
    deprecated_keys: serde_json::Value,
    last_seen: Option<chrono::DateTime<chrono::Utc>>,
}
