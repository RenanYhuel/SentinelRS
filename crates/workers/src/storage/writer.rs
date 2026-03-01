use sqlx::PgPool;

use crate::transform::MetricRow;
use super::retry::WriteError;

pub struct MetricWriter {
    pool: PgPool,
}

impl MetricWriter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_batch(&self, rows: &[MetricRow]) -> Result<u64, WriteError> {
        if rows.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        let mut inserted = 0u64;

        for row in rows {
            let labels_json = serde_json::to_value(&row.labels)?;
            let hist_bounds = row.histogram_boundaries.as_deref();
            let hist_counts: Option<Vec<i64>> = row
                .histogram_counts
                .as_ref()
                .map(|v| v.iter().map(|c| *c as i64).collect());
            let hist_count = row.histogram_count.map(|c| c as i64);

            sqlx::query(
                r#"INSERT INTO metrics_raw
                   (time, agent_id, name, labels, metric_type, value,
                    histogram_boundaries, histogram_counts, histogram_count, histogram_sum)
                   VALUES
                   (to_timestamp($1::double precision / 1000), $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
            )
            .bind(row.time_ms as f64)
            .bind(&row.agent_id)
            .bind(&row.name)
            .bind(&labels_json)
            .bind(&row.metric_type)
            .bind(row.value)
            .bind(hist_bounds)
            .bind(hist_counts.as_deref())
            .bind(hist_count)
            .bind(row.histogram_sum)
            .execute(&mut *tx)
            .await?;

            inserted += 1;
        }

        tx.commit().await?;
        Ok(inserted)
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
