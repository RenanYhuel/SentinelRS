use sqlx::PgPool;

use super::loader;
use super::tracker;

pub async fn run(pool: &PgPool) -> Result<usize, Box<dyn std::error::Error>> {
    tracker::ensure_tracking_table(pool).await?;

    let migrations = loader::load_all();
    let mut applied_count = 0;

    for migration in &migrations {
        if tracker::is_applied(pool, migration.filename).await? {
            continue;
        }

        tracing::info!(file = migration.filename, "applying migration");

        for statement in split_statements(migration.sql) {
            let trimmed = statement.trim();
            if trimmed.is_empty() {
                continue;
            }
            sqlx::query(trimmed)
                .execute(pool)
                .await
                .map_err(|e| format!("migration {} failed: {}", migration.filename, e))?;
        }

        tracker::mark_applied(pool, migration.filename).await?;
        applied_count += 1;
    }

    Ok(applied_count)
}

fn split_statements(sql: &str) -> Vec<&str> {
    sql.split(';').collect()
}
