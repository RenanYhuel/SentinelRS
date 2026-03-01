use sentinel_common::nats_config::StreamConfig;
use sentinel_common::trace_id::generate_trace_id;
use sentinel_workers::api;
use sentinel_workers::consumer::{ConsumerLoop, connect_jetstream, create_pull_consumer, ensure_stream};
use sentinel_workers::metrics::worker_metrics::WorkerMetrics;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .json()
        .init();

    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".into());
    let batch_size: usize = std::env::var("BATCH_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);
    let api_addr = std::env::var("WORKER_API_ADDR").unwrap_or_else(|_| "0.0.0.0:9090".into());

    let worker_metrics = WorkerMetrics::new();

    let api_metrics = worker_metrics.clone();
    let api_handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&api_addr).await.unwrap();
        tracing::info!(%api_addr, "worker API server starting");
        api::serve(listener, api_metrics).await.unwrap();
    });

    tracing::info!(url = %nats_url, "connecting to NATS JetStream");
    let js = connect_jetstream(&nats_url).await?;

    let stream_config = StreamConfig::default();
    ensure_stream(&js, &stream_config).await?;
    tracing::info!(stream = %stream_config.name, "stream ready");

    let consumer = create_pull_consumer(&js).await?;
    tracing::info!("pull consumer ready, entering loop");

    let consumer_loop = ConsumerLoop::new(consumer, batch_size);
    let consumer_handle = tokio::spawn(async move {
        consumer_loop
            .run(|batch, batch_id| async move {
                let trace_id = generate_trace_id();
                let bid = batch_id.as_deref().unwrap_or("unknown");
                let _span = tracing::info_span!("process_batch", %trace_id, batch_id = bid, agent_id = %batch.agent_id).entered();
                tracing::info!(metrics = batch.metrics.len(), "received batch (processing stub)");
                Ok(())
            })
            .await
    });

    tokio::select! {
        r = api_handle => { if let Err(e) = r { tracing::error!("API: {e}"); } }
        r = consumer_handle => {
            match r {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::error!("consumer: {e}"),
                Err(e) => tracing::error!("consumer join: {e}"),
            }
        }
    }

    Ok(())
}
