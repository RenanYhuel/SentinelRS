use sqlx::PgPool;

use sentinel_workers::storage::migrator;
use sentinel_workers::storage::{create_pool, MetricWriter, RawWriter};
use sentinel_workers::transform::MetricRow;
use std::collections::HashMap;

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/sentinel".into());
    create_pool(&url, 2).await.expect("connect to TimescaleDB")
}

async fn clean_tables(pool: &PgPool) {
    let _ = sqlx::raw_sql(
        "DROP TABLE IF EXISTS metrics_time CASCADE;
         DROP TABLE IF EXISTS metrics_raw CASCADE;
         DROP TABLE IF EXISTS alerts CASCADE;
         DROP TABLE IF EXISTS alert_rules CASCADE;
         DROP TABLE IF EXISTS notifications_dlq CASCADE;
         DROP TABLE IF EXISTS _migrations CASCADE;",
    )
    .execute(pool)
    .await;
}

#[tokio::test]
#[ignore]
async fn migrations_run_idempotently() {
    let pool = setup_pool().await;
    clean_tables(&pool).await;

    let first = migrator::run_migrations(&pool).await.unwrap();
    assert!(!first.is_empty());

    let second = migrator::run_migrations(&pool).await.unwrap();
    assert!(second.is_empty());
}

#[tokio::test]
#[ignore]
async fn pending_migrations_detects_unapplied() {
    let pool = setup_pool().await;
    clean_tables(&pool).await;

    let pending = migrator::pending_migrations(&pool).await.unwrap();
    assert!(!pending.is_empty());

    migrator::run_migrations(&pool).await.unwrap();

    let pending_after = migrator::pending_migrations(&pool).await.unwrap();
    assert!(pending_after.is_empty());
}

#[tokio::test]
#[ignore]
async fn insert_metric_row_and_query() {
    let pool = setup_pool().await;
    migrator::run_migrations(&pool).await.unwrap();

    let writer = MetricWriter::new(pool.clone());
    let row = MetricRow {
        time_ms: 1_700_000_000_000,
        agent_id: "test-agent".into(),
        name: "cpu.usage".into(),
        labels: {
            let mut m = HashMap::new();
            m.insert("host".into(), "srv-1".into());
            m
        },
        metric_type: "gauge".into(),
        value: Some(72.5),
        histogram_boundaries: None,
        histogram_counts: None,
        histogram_count: None,
        histogram_sum: None,
    };

    let inserted = writer.insert_batch(&[row]).await.unwrap();
    assert_eq!(inserted, 1);

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM metrics_time WHERE agent_id = 'test-agent' AND name = 'cpu.usage'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(count.0 >= 1);

    sqlx::raw_sql("DELETE FROM metrics_time WHERE agent_id = 'test-agent'")
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore]
async fn insert_raw_batch_and_query() {
    let pool = setup_pool().await;
    migrator::run_migrations(&pool).await.unwrap();

    let raw = RawWriter::new(pool.clone());
    let payload = serde_json::json!({
        "metrics": [{"name": "mem.free", "value": 1024}]
    });
    raw.insert_raw_batch("test-agent", "batch-001", &payload)
        .await
        .unwrap();

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM metrics_raw WHERE agent_id = 'test-agent' AND batch_id = 'batch-001'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(count.0 >= 1);

    sqlx::raw_sql("DELETE FROM metrics_raw WHERE agent_id = 'test-agent'")
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore]
async fn v_recent_values_returns_latest() {
    let pool = setup_pool().await;
    migrator::run_migrations(&pool).await.unwrap();

    let writer = MetricWriter::new(pool.clone());
    let rows = vec![
        MetricRow {
            time_ms: 1_700_000_000_000,
            agent_id: "view-agent".into(),
            name: "disk.free".into(),
            labels: HashMap::new(),
            metric_type: "gauge".into(),
            value: Some(100.0),
            histogram_boundaries: None,
            histogram_counts: None,
            histogram_count: None,
            histogram_sum: None,
        },
        MetricRow {
            time_ms: 1_700_000_001_000,
            agent_id: "view-agent".into(),
            name: "disk.free".into(),
            labels: HashMap::new(),
            metric_type: "gauge".into(),
            value: Some(95.0),
            histogram_boundaries: None,
            histogram_counts: None,
            histogram_count: None,
            histogram_sum: None,
        },
    ];
    writer.insert_batch(&rows).await.unwrap();

    let latest: (f64,) = sqlx::query_as(
        "SELECT value FROM v_recent_values WHERE agent_id = 'view-agent' AND name = 'disk.free'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!((latest.0 - 95.0).abs() < 0.01);

    sqlx::raw_sql("DELETE FROM metrics_time WHERE agent_id = 'view-agent'")
        .execute(&pool)
        .await
        .unwrap();
}
