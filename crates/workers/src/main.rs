use std::sync::Arc;

use sentinel_common::logging::{self, Component, LogConfig};
use sentinel_common::nats_config::StreamConfig;
use sentinel_common::trace_id::generate_trace_id;
use sentinel_workers::api;
use sentinel_workers::consumer::{
    connect_jetstream, create_pull_consumer, ensure_stream, ConsumerLoop,
};
use sentinel_workers::ingestion::IngestPipeline;
use sentinel_workers::metrics::worker_metrics::WorkerMetrics;
use sentinel_workers::storage::{create_pool, migrator};
use tracing::Instrument;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let log_config = LogConfig::from_env();
    logging::print_banner(Component::Worker, env!("CARGO_PKG_VERSION"));
    logging::init(&log_config);

    tracing::info!(target: "system", "Starting SentinelRS Worker");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let max_db_connections: u32 = std::env::var("MAX_DB_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".into());
    let batch_size: usize = std::env::var("BATCH_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);
    let api_addr = std::env::var("WORKER_API_ADDR").unwrap_or_else(|_| "0.0.0.0:9090".into());

    let sw = logging::stopwatch();
    let pool = create_pool(&database_url, max_db_connections).await?;
    tracing::info!(target: "db", "Database pool created{sw}");

    let applied = migrator::run_migrations(&pool).await?;
    if !applied.is_empty() {
        tracing::info!(target: "db", count = applied.len(), "Migrations applied");
    } else {
        tracing::info!(target: "db", "Schema up to date");
    }

    let worker_metrics = WorkerMetrics::new();
    let pipeline = Arc::new(IngestPipeline::new(pool, worker_metrics.clone()));

    let api_metrics = worker_metrics.clone();
    let api_handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&api_addr).await.unwrap();
        tracing::info!(target: "net", "Worker API listening on {api_addr}");
        api::serve(listener, api_metrics).await.unwrap();
    });

    let sw = logging::stopwatch();
    let js = connect_jetstream(&nats_url).await?;
    tracing::info!(target: "net", "NATS JetStream connected ({nats_url}){sw}");

    let stream_config = StreamConfig::default();
    ensure_stream(&js, &stream_config).await?;
    tracing::info!(target: "net", "Stream '{}' ready", stream_config.name);

    let consumer = create_pull_consumer(&js).await?;
    tracing::info!(target: "work", "Pull consumer ready — entering processing loop");

    tracing::info!(target: "system", "Worker ready");

    let consumer_loop = ConsumerLoop::new(consumer, batch_size);
    let consumer_handle = tokio::spawn(async move {
        consumer_loop
            .run(|batch, batch_id| {
                let pipeline = Arc::clone(&pipeline);
                async move {
                    let trace_id = generate_trace_id();
                    let bid = batch_id.as_deref().unwrap_or("unknown");
                    let span = tracing::info_span!("process_batch", %trace_id, batch_id = bid, agent_id = %batch.agent_id);
                    pipeline.ingest(&batch).instrument(span).await
                }
            })
            .await
    });

    tokio::select! {
        r = api_handle => { if let Err(e) = r { tracing::error!(target: "net", "API: {e}"); } }
        r = consumer_handle => {
            match r {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::error!(target: "work", "Consumer: {e}"),
                Err(e) => tracing::error!(target: "work", "Consumer join: {e}"),
            }
        }
    }

    Ok(())
}
