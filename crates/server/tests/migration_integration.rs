use sqlx::PgPool;

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/sentinel".into());
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect to PostgreSQL")
}

async fn drop_tracking_table(pool: &PgPool) {
    let _ = sqlx::raw_sql("DROP TABLE IF EXISTS _migration_tracking CASCADE;")
        .execute(pool)
        .await;
}

#[tokio::test]
#[ignore]
async fn run_migrations_on_empty_db() {
    let pool = setup_pool().await;
    drop_tracking_table(&pool).await;

    let applied = sentinel_server::migration::run(&pool).await.unwrap();
    assert!(applied > 0, "at least one migration should be applied");

    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM _migration_tracking")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(row.0 > 0);
}

#[tokio::test]
#[ignore]
async fn run_migrations_idempotent() {
    let pool = setup_pool().await;
    drop_tracking_table(&pool).await;

    let first = sentinel_server::migration::run(&pool).await.unwrap();
    assert!(first > 0);

    let second = sentinel_server::migration::run(&pool).await.unwrap();
    assert_eq!(second, 0, "re-running should apply zero migrations");
}
