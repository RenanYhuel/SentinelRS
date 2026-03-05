use sentinel_common::pool_config::PoolConfig;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(database_url: &str, config: &PoolConfig) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .idle_timeout(config.idle_timeout)
        .acquire_timeout(config.acquire_timeout)
        .max_lifetime(config.max_lifetime)
        .connect(database_url)
        .await
}
