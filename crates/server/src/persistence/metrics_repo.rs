use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Clone)]
pub struct MetricsQueryRepo {
    pool: PgPool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LatestMetric {
    pub agent_id: String,
    pub name: String,
    pub value: Option<f64>,
    pub time: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TimeSeriesPoint {
    pub bucket: DateTime<Utc>,
    pub avg_value: Option<f64>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub sample_count: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MetricNameRow {
    pub name: String,
    pub total_samples: Option<i64>,
    pub last_seen: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AgentSummaryRow {
    pub agent_id: String,
    pub metric_count: Option<i64>,
    pub sample_count: Option<i64>,
    pub latest_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TopMetricRow {
    pub name: String,
    pub agent_id: String,
    pub total_samples: Option<i64>,
    pub last_seen: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ComparePoint {
    pub bucket: DateTime<Utc>,
    pub agent_id: String,
    pub avg_value: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PercentileResult {
    pub p50: Option<f64>,
    pub p90: Option<f64>,
    pub p99: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub count: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct PercentileRow {
    p50: Option<f64>,
    p90: Option<f64>,
    p99: Option<f64>,
    min_val: Option<f64>,
    max_val: Option<f64>,
    cnt: Option<i64>,
}

impl MetricsQueryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn latest(&self, agent_id: &str) -> Result<Vec<LatestMetric>, sqlx::Error> {
        sqlx::query_as::<_, LatestMetric>(
            "SELECT agent_id, name, value, time FROM v_recent_values WHERE agent_id = $1",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn history(
        &self,
        agent_id: &str,
        metric: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        interval: &str,
    ) -> Result<Vec<TimeSeriesPoint>, sqlx::Error> {
        let valid_interval = match interval {
            "1m" => "1 minute",
            "5m" => "5 minutes",
            "15m" => "15 minutes",
            "1h" => "1 hour",
            "6h" => "6 hours",
            "1d" => "1 day",
            _ => "5 minutes",
        };

        let query = format!(
            "SELECT time_bucket('{valid_interval}', time) AS bucket,
                    AVG(value) AS avg_value,
                    MIN(value) AS min_value,
                    MAX(value) AS max_value,
                    COUNT(*) AS sample_count
             FROM metrics_time
             WHERE agent_id = $1 AND name = $2
                   AND time >= $3 AND time <= $4
                   AND value IS NOT NULL
             GROUP BY bucket
             ORDER BY bucket"
        );

        sqlx::query_as::<_, TimeSeriesPoint>(&query)
            .bind(agent_id)
            .bind(metric)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn history_1h(
        &self,
        agent_id: &str,
        metric: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<TimeSeriesPoint>, sqlx::Error> {
        sqlx::query_as::<_, TimeSeriesPoint>(
            "SELECT bucket, avg_value, min_value, max_value, sample_count
             FROM mv_metrics_1h
             WHERE agent_id = $1 AND name = $2
                   AND bucket >= $3 AND bucket <= $4
             ORDER BY bucket",
        )
        .bind(agent_id)
        .bind(metric)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn history_5m(
        &self,
        agent_id: &str,
        metric: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<TimeSeriesPoint>, sqlx::Error> {
        sqlx::query_as::<_, TimeSeriesPoint>(
            "SELECT bucket, avg_value, min_value, max_value, sample_count
             FROM mv_metrics_5m
             WHERE agent_id = $1 AND name = $2
                   AND bucket >= $3 AND bucket <= $4
             ORDER BY bucket",
        )
        .bind(agent_id)
        .bind(metric)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn metric_names(&self, agent_id: &str) -> Result<Vec<MetricNameRow>, sqlx::Error> {
        sqlx::query_as::<_, MetricNameRow>(
            "SELECT name, total_samples, last_seen
             FROM v_top_metrics
             WHERE agent_id = $1
             ORDER BY total_samples DESC",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn summary(&self) -> Result<Vec<AgentSummaryRow>, sqlx::Error> {
        sqlx::query_as::<_, AgentSummaryRow>(
            "SELECT agent_id,
                    COUNT(DISTINCT name) AS metric_count,
                    SUM(total_samples) AS sample_count,
                    MAX(last_seen) AS latest_time
             FROM v_top_metrics
             GROUP BY agent_id
             ORDER BY latest_time DESC NULLS LAST",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn top(&self, agent_id: &str, limit: i64) -> Result<Vec<TopMetricRow>, sqlx::Error> {
        sqlx::query_as::<_, TopMetricRow>(
            "SELECT name, agent_id, total_samples, last_seen
             FROM v_top_metrics
             WHERE agent_id = $1
             ORDER BY total_samples DESC
             LIMIT $2",
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn compare(
        &self,
        agent_ids: &[String],
        metric: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        interval: &str,
    ) -> Result<Vec<ComparePoint>, sqlx::Error> {
        let valid_interval = match interval {
            "1m" => "1 minute",
            "5m" => "5 minutes",
            "15m" => "15 minutes",
            "1h" => "1 hour",
            "6h" => "6 hours",
            "1d" => "1 day",
            _ => "5 minutes",
        };

        let query = format!(
            "SELECT time_bucket('{valid_interval}', time) AS bucket,
                    agent_id,
                    AVG(value) AS avg_value
             FROM metrics_time
             WHERE agent_id = ANY($1) AND name = $2
                   AND time >= $3 AND time <= $4
                   AND value IS NOT NULL
             GROUP BY bucket, agent_id
             ORDER BY bucket, agent_id"
        );

        sqlx::query_as::<_, ComparePoint>(&query)
            .bind(agent_ids)
            .bind(metric)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn percentiles(
        &self,
        agent_id: &str,
        metric: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<PercentileResult, sqlx::Error> {
        let row = sqlx::query_as::<_, PercentileRow>(
            "SELECT
                percentile_cont(0.50) WITHIN GROUP (ORDER BY value) AS p50,
                percentile_cont(0.90) WITHIN GROUP (ORDER BY value) AS p90,
                percentile_cont(0.99) WITHIN GROUP (ORDER BY value) AS p99,
                MIN(value) AS min_val,
                MAX(value) AS max_val,
                COUNT(*) AS cnt
             FROM metrics_time
             WHERE agent_id = $1 AND name = $2
                   AND time >= $3 AND time <= $4
                   AND value IS NOT NULL",
        )
        .bind(agent_id)
        .bind(metric)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await?;

        Ok(PercentileResult {
            p50: row.p50,
            p90: row.p90,
            p99: row.p99,
            min: row.min_val,
            max: row.max_val,
            count: row.cnt.unwrap_or(0),
        })
    }

    pub async fn export_raw(
        &self,
        agent_id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        metric: Option<&str>,
    ) -> Result<Vec<LatestMetric>, sqlx::Error> {
        match metric {
            Some(name) => {
                sqlx::query_as::<_, LatestMetric>(
                    "SELECT agent_id, name, value, time
                     FROM metrics_time
                     WHERE agent_id = $1 AND name = $2
                           AND time >= $3 AND time <= $4
                     ORDER BY time",
                )
                .bind(agent_id)
                .bind(name)
                .bind(from)
                .bind(to)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, LatestMetric>(
                    "SELECT agent_id, name, value, time
                     FROM metrics_time
                     WHERE agent_id = $1
                           AND time >= $2 AND time <= $3
                     ORDER BY time",
                )
                .bind(agent_id)
                .bind(from)
                .bind(to)
                .fetch_all(&self.pool)
                .await
            }
        }
    }
}
