use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub struct HealthConfig {
    pub timeout: Duration,
    pub retry_interval: Duration,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            retry_interval: Duration::from_millis(500),
        }
    }
}

pub async fn wait_for_db(
    database_url: &str,
    max_connections: u32,
    config: &HealthConfig,
) -> Result<PgPool, String> {
    let deadline = tokio::time::Instant::now() + config.timeout;

    loop {
        match PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(3))
            .connect(database_url)
            .await
        {
            Ok(pool) => {
                if sqlx::query("SELECT 1").execute(&pool).await.is_ok() {
                    return Ok(pool);
                }
                drop(pool);
            }
            Err(e) => {
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!(
                        "database not ready after {:?}: {}",
                        config.timeout, e
                    ));
                }
                tracing::warn!(
                    error = %e,
                    retry_in_ms = config.retry_interval.as_millis() as u64,
                    "database not ready, retrying"
                );
            }
        }

        tokio::time::sleep(config.retry_interval).await;

        if tokio::time::Instant::now() >= deadline {
            return Err(format!("database not ready after {:?}", config.timeout));
        }
    }
}
