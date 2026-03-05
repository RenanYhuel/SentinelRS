use std::sync::Arc;
use std::time::Instant;

use sentinel_common::proto::Batch;
use sqlx::PgPool;

use super::alert_engine::AlertEngine;
use crate::metrics::worker_metrics::WorkerMetrics;
use crate::storage::{write_with_retry, AgentRepo, MetricWriter};
use crate::transform::transform_batch;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const MAX_RETRIES: u32 = 3;

pub struct IngestPipeline {
    writer: MetricWriter,
    agent_repo: AgentRepo,
    metrics: Arc<WorkerMetrics>,
    alert_engine: Option<Arc<AlertEngine>>,
}

impl IngestPipeline {
    pub fn new(pool: PgPool, metrics: Arc<WorkerMetrics>) -> Self {
        Self {
            writer: MetricWriter::new(pool.clone()),
            agent_repo: AgentRepo::new(pool),
            metrics,
            alert_engine: None,
        }
    }

    pub fn with_alert_engine(mut self, engine: Arc<AlertEngine>) -> Self {
        self.alert_engine = Some(engine);
        self
    }

    pub async fn ingest(&self, batch: &Batch) -> Result<(), BoxError> {
        let start = Instant::now();

        let rows = transform_batch(batch);
        if rows.is_empty() {
            tracing::debug!(agent_id = %batch.agent_id, "empty batch, skipping");
            return Ok(());
        }

        let db_start = Instant::now();
        let inserted = write_with_retry(&self.writer, &rows, MAX_RETRIES).await?;
        let db_latency_ms = db_start.elapsed().as_millis() as u64;
        self.metrics.record_db_latency(db_start);
        self.metrics.add_rows_inserted(inserted);

        if db_latency_ms > 500 {
            sentinel_common::logging::latency::warn_slow("db_write", db_latency_ms, 500);
        }

        self.agent_repo.touch_last_seen(&batch.agent_id).await?;

        if let Some(ref engine) = self.alert_engine {
            engine.process(&batch.agent_id, &rows).await;
        }

        self.metrics.inc_batches_processed();
        self.metrics.record_processing_latency(start);

        let latency_ms = start.elapsed().as_millis() as u64;
        tracing::debug!(
            target: "data",
            agent_id = %batch.agent_id,
            inserted,
            latency_ms,
            "Batch ingested"
        );

        Ok(())
    }
}
