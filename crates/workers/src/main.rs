use std::sync::Arc;

use sentinel_common::logging::{self, Component, LogConfig};
use sentinel_common::nats_config::StreamConfig;
use sentinel_common::trace_id::generate_trace_id;
use sentinel_workers::api;
use sentinel_workers::api::state::WorkerState;
use sentinel_workers::backpressure::{BatchSemaphore, CircuitBreaker};
use sentinel_workers::config::WorkerConfig;
use sentinel_workers::consumer::{
    connect_jetstream, create_group_consumer, ensure_stream, ConsumerLoop,
};
use sentinel_workers::identity::WorkerIdentity;
use sentinel_workers::ingestion::IngestPipeline;
use sentinel_workers::metrics::worker_metrics::WorkerMetrics;
use sentinel_workers::registry::WorkerRegistry;
use sentinel_workers::shutdown::spawn_signal_handler;
use sentinel_workers::storage::{create_pool, migrator};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let log_config = LogConfig::from_env();
    logging::print_banner(Component::Worker, env!("CARGO_PKG_VERSION"));
    logging::init(&log_config);

    let config = WorkerConfig::from_env();
    let identity = WorkerIdentity::generate();
    let cancel = CancellationToken::new();

    tracing::info!(
        target: "system",
        worker_id = identity.id(),
        "Starting SentinelRS Worker"
    );

    spawn_signal_handler(cancel.clone());

    let sw = logging::stopwatch();
    let pool = create_pool(&config.database_url, config.max_db_connections).await?;
    tracing::info!(target: "db", "Database pool created{sw}");

    let applied = migrator::run_migrations(&pool).await?;
    if !applied.is_empty() {
        tracing::info!(target: "db", count = applied.len(), "Migrations applied");
    } else {
        tracing::info!(target: "db", "Schema up to date");
    }

    let worker_metrics = WorkerMetrics::new();
    let circuit_breaker = CircuitBreaker::new(
        config.backpressure.circuit_breaker_threshold,
        config.backpressure.circuit_breaker_reset,
    );
    let semaphore = Arc::new(BatchSemaphore::new(
        config.backpressure.max_concurrent_batches,
    ));
    let pipeline = Arc::new(IngestPipeline::new(pool, worker_metrics.clone()));

    let sw = logging::stopwatch();
    let (js, _client) = connect_jetstream(&config.nats_url).await?;
    tracing::info!(target: "net", nats_url = %config.nats_url, "NATS JetStream connected{sw}");

    let stream_config = StreamConfig::default();
    ensure_stream(&js, &stream_config).await?;
    tracing::info!(target: "net", stream = %stream_config.name, "Stream ready");

    let consumer = create_group_consumer(&js, &config.consumer_group).await?;
    tracing::info!(
        target: "work",
        group = %config.consumer_group.group_name,
        max_ack_pending = config.consumer_group.max_ack_pending,
        "Consumer group joined"
    );

    let registry = if config.registry.enabled {
        match WorkerRegistry::create(js.clone(), &config.registry, Arc::clone(&identity)).await {
            Ok(reg) => {
                reg.spawn_heartbeat(config.registry.heartbeat_interval, cancel.clone());
                tracing::info!(
                    target: "registry",
                    bucket = %config.registry.bucket,
                    "Worker registry active"
                );
                Some(reg)
            }
            Err(e) => {
                tracing::warn!(target: "registry", error = %e, "Registry init failed, continuing without peer discovery");
                None
            }
        }
    } else {
        None
    };

    let consumer_loop = ConsumerLoop::new(consumer, config.batch_size, cancel.clone());
    let in_flight = consumer_loop.in_flight();

    let worker_state = Arc::new(WorkerState {
        identity: Arc::clone(&identity),
        metrics: worker_metrics.clone(),
        circuit_breaker: Arc::clone(&circuit_breaker),
        semaphore: Arc::clone(&semaphore),
        in_flight: Arc::clone(&in_flight),
        registry,
    });

    let api_addr = config.api_addr.clone();
    let api_state = Arc::clone(&worker_state);
    let api_handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&api_addr).await.unwrap();
        tracing::info!(target: "net", addr = %api_addr, "Worker API listening");
        api::serve(listener, api_state).await.unwrap();
    });

    tracing::info!(
        target: "system",
        worker_id = identity.id(),
        "Worker ready — entering processing loop"
    );

    let cb = Arc::clone(&circuit_breaker);
    let consumer_handle = tokio::spawn(async move {
        consumer_loop
            .run(|batch, batch_id| {
                let pipeline = Arc::clone(&pipeline);
                let cb = Arc::clone(&cb);
                async move {
                    if !cb.allow().await {
                        tracing::warn!(target: "backpressure", "Circuit breaker open — skipping batch");
                        return Err("circuit breaker open".into());
                    }

                    let trace_id = generate_trace_id();
                    let bid = batch_id.as_deref().unwrap_or("unknown");
                    let span = tracing::info_span!(
                        "process_batch",
                        %trace_id,
                        batch_id = bid,
                        agent_id = %batch.agent_id
                    );

                    let result = pipeline.ingest(&batch).instrument(span).await;

                    match &result {
                        Ok(()) => cb.record_success().await,
                        Err(_) => cb.record_failure().await,
                    }

                    result
                }
            })
            .await
    });

    tokio::select! {
        r = api_handle => {
            if let Err(e) = r {
                tracing::error!(target: "net", error = %e, "API task failed");
            }
        }
        r = consumer_handle => {
            match r {
                Ok(Ok(())) => tracing::info!(target: "work", "Consumer loop exited cleanly"),
                Ok(Err(e)) => tracing::error!(target: "work", error = %e, "Consumer error"),
                Err(e) => tracing::error!(target: "work", error = %e, "Consumer join error"),
            }
        }
    }

    tracing::info!(
        target: "system",
        worker_id = identity.id(),
        uptime = %identity.uptime_human(),
        "Worker shutdown complete"
    );

    Ok(())
}
