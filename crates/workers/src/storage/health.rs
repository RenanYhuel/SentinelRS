use std::time::Duration;

use sentinel_common::pool_config::PoolConfig;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub struct WaitConfig {
    pub timeout: Duration,
    pub retry_interval: Duration,
    pub max_retries: u32,
}

impl Default for WaitConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            retry_interval: Duration::from_millis(1000),
            max_retries: 60,
        }
    }
}

impl WaitConfig {
    pub fn from_env() -> Self {
        Self {
            timeout: Duration::from_secs(env_parse("DB_WAIT_TIMEOUT_SECS", 120)),
            retry_interval: Duration::from_millis(env_parse("DB_WAIT_RETRY_INTERVAL_MS", 1000)),
            max_retries: env_parse("DB_WAIT_MAX_RETRIES", 60),
        }
    }
}

pub async fn wait_for_db(
    database_url: &str,
    pool_config: &PoolConfig,
    wait_config: &WaitConfig,
) -> Result<PgPool, String> {
    let deadline = tokio::time::Instant::now() + wait_config.timeout;
    let mut attempt: u32 = 0;

    tracing::info!(
        target: "db",
        timeout_secs = wait_config.timeout.as_secs(),
        max_retries = wait_config.max_retries,
        "Waiting for database"
    );

    loop {
        attempt += 1;

        match PgPoolOptions::new()
            .max_connections(pool_config.max_connections)
            .min_connections(pool_config.min_connections)
            .idle_timeout(pool_config.idle_timeout)
            .acquire_timeout(pool_config.acquire_timeout)
            .max_lifetime(pool_config.max_lifetime)
            .connect(database_url)
            .await
        {
            Ok(pool) => match sqlx::query("SELECT 1").execute(&pool).await {
                Ok(_) => {
                    tracing::info!(
                        target: "db",
                        attempts = attempt,
                        "Database ready"
                    );
                    return Ok(pool);
                }
                Err(e) => {
                    tracing::warn!(
                        target: "db",
                        attempt,
                        max_retries = wait_config.max_retries,
                        error = %e,
                        "Connected but SELECT 1 failed"
                    );
                    drop(pool);
                }
            },
            Err(e) => {
                tracing::warn!(
                    target: "db",
                    attempt,
                    max_retries = wait_config.max_retries,
                    retry_in_ms = wait_config.retry_interval.as_millis() as u64,
                    error = %e,
                    "Database not ready, retrying"
                );
            }
        }

        if attempt >= wait_config.max_retries || tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "database not ready after {} attempts / {:?}",
                attempt, wait_config.timeout
            ));
        }

        tokio::time::sleep(wait_config.retry_interval).await;
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
