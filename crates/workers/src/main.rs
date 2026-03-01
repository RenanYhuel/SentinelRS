use sentinel_common::nats_config::StreamConfig;
use sentinel_workers::consumer::{ConsumerLoop, connect_jetstream, create_pull_consumer, ensure_stream};
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

    tracing::info!(url = %nats_url, "connecting to NATS JetStream");
    let js = connect_jetstream(&nats_url).await?;

    let stream_config = StreamConfig::default();
    ensure_stream(&js, &stream_config).await?;
    tracing::info!(stream = %stream_config.name, "stream ready");

    let consumer = create_pull_consumer(&js).await?;
    tracing::info!("pull consumer ready, entering loop");

    let consumer_loop = ConsumerLoop::new(consumer, batch_size);
    consumer_loop
        .run(|batch, batch_id| async move {
            tracing::info!(
                batch_id = batch_id.as_deref().unwrap_or("unknown"),
                agent_id = %batch.agent_id,
                metrics = batch.metrics.len(),
                "received batch (processing stub)"
            );
            Ok(())
        })
        .await?;

    Ok(())
}
