use sqlx::PgPool;

pub async fn ensure_tracking_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id          SERIAL PRIMARY KEY,
            filename    TEXT   NOT NULL UNIQUE,
            applied_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn is_applied(pool: &PgPool, filename: &str) -> Result<bool, sqlx::Error> {
    let row: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM _migrations WHERE filename = $1)")
            .bind(filename)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

pub async fn mark_applied(pool: &PgPool, filename: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO _migrations (filename) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(filename)
        .execute(pool)
        .await?;
    Ok(())
}
