use std::time::Duration;

use sentinel_common::pool_config::PoolConfig;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub struct HealthConfig {
    pub timeout: Duration,
    pub retry_interval: Duration,
    pub max_retries: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            retry_interval: Duration::from_millis(1000),
            max_retries: 60,
        }
    }
}

impl HealthConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("DB_WAIT_TIMEOUT_SECS") {
            if let Ok(secs) = v.parse::<u64>() {
                cfg.timeout = Duration::from_secs(secs);
            }
        }
        if let Ok(v) = std::env::var("DB_WAIT_RETRY_INTERVAL_MS") {
            if let Ok(ms) = v.parse::<u64>() {
                cfg.retry_interval = Duration::from_millis(ms);
            }
        }
        if let Ok(v) = std::env::var("DB_WAIT_MAX_RETRIES") {
            if let Ok(n) = v.parse::<u32>() {
                cfg.max_retries = n;
            }
        }
        cfg
    }
}

pub async fn wait_for_db(
    database_url: &str,
    pool_config: &PoolConfig,
    health_config: &HealthConfig,
) -> Result<PgPool, String> {
    let deadline = tokio::time::Instant::now() + health_config.timeout;
    let mut attempt: u32 = 0;

    tracing::info!(
        target: "db",
        timeout_secs = health_config.timeout.as_secs(),
        max_retries = health_config.max_retries,
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
                        max_retries = health_config.max_retries,
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
                    max_retries = health_config.max_retries,
                    retry_in_ms = health_config.retry_interval.as_millis() as u64,
                    error = %e,
                    "Database not ready, retrying"
                );
            }
        }

        if attempt >= health_config.max_retries || tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "database not ready after {} attempts / {:?}",
                attempt, health_config.timeout
            ));
        }

        tokio::time::sleep(health_config.retry_interval).await;
    }
}
