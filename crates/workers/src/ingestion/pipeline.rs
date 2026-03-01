use std::sync::Arc;
use std::time::Instant;

use sentinel_common::proto::Batch;
use sqlx::PgPool;

use crate::metrics::worker_metrics::WorkerMetrics;
use crate::storage::{write_with_retry, AgentRepo, MetricWriter};
use crate::transform::transform_batch;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const MAX_RETRIES: u32 = 3;

pub struct IngestPipeline {
    writer: MetricWriter,
    agent_repo: AgentRepo,
    metrics: Arc<WorkerMetrics>,
}

impl IngestPipeline {
    pub fn new(pool: PgPool, metrics: Arc<WorkerMetrics>) -> Self {
        Self {
            writer: MetricWriter::new(pool.clone()),
            agent_repo: AgentRepo::new(pool),
            metrics,
        }
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
        self.metrics.record_db_latency(db_start);
        self.metrics.add_rows_inserted(inserted);

        self.agent_repo.touch_last_seen(&batch.agent_id).await?;

        self.metrics.inc_batches_processed();
        self.metrics.record_processing_latency(start);

        tracing::info!(
            agent_id = %batch.agent_id,
            inserted,
            "batch ingested"
        );

        Ok(())
    }
}
