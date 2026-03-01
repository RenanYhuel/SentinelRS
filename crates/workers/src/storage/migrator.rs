use sqlx::PgPool;

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "000_migration_tracking.sql",
        include_str!("../../../../migrations/000_migration_tracking.sql"),
    ),
    (
        "001_create_extensions.sql",
        include_str!("../../../../migrations/001_create_extensions.sql"),
    ),
    (
        "002_create_metrics_time.sql",
        include_str!("../../../../migrations/002_create_metrics_time.sql"),
    ),
    (
        "003_create_metrics_raw.sql",
        include_str!("../../../../migrations/003_create_metrics_raw.sql"),
    ),
    (
        "004_create_alerts.sql",
        include_str!("../../../../migrations/004_create_alerts.sql"),
    ),
    (
        "005_retention_policies.sql",
        include_str!("../../../../migrations/005_retention_policies.sql"),
    ),
    (
        "006_continuous_aggregates.sql",
        include_str!("../../../../migrations/006_continuous_aggregates.sql"),
    ),
    (
        "007_dashboard_views.sql",
        include_str!("../../../../migrations/007_dashboard_views.sql"),
    ),
    (
        "008_create_alert_rules.sql",
        include_str!("../../../../migrations/008_create_alert_rules.sql"),
    ),
    (
        "009_create_notifications_dlq.sql",
        include_str!("../../../../migrations/009_create_notifications_dlq.sql"),
    ),
];

pub async fn run_migrations(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    let bootstrap = MIGRATIONS[0].1;
    sqlx::raw_sql(bootstrap).execute(pool).await?;

    let applied: Vec<String> = sqlx::query_scalar("SELECT filename FROM _migrations")
        .fetch_all(pool)
        .await?;

    let mut newly_applied = Vec::new();

    for (filename, sql) in &MIGRATIONS[1..] {
        if applied.iter().any(|a| a == filename) {
            continue;
        }
        sqlx::raw_sql(sql).execute(pool).await?;
        sqlx::query("INSERT INTO _migrations (filename) VALUES ($1)")
            .bind(filename)
            .execute(pool)
            .await?;
        newly_applied.push(filename.to_string());
    }

    Ok(newly_applied)
}

pub async fn pending_migrations(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    let bootstrap = MIGRATIONS[0].1;
    sqlx::raw_sql(bootstrap).execute(pool).await?;

    let applied: Vec<String> = sqlx::query_scalar("SELECT filename FROM _migrations")
        .fetch_all(pool)
        .await?;

    let pending: Vec<String> = MIGRATIONS[1..]
        .iter()
        .filter(|(name, _)| !applied.iter().any(|a| a == name))
        .map(|(name, _)| name.to_string())
        .collect();

    Ok(pending)
}
