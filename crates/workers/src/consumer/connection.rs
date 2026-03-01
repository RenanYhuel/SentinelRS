use async_nats::jetstream;
use async_nats::jetstream::consumer::PullConsumer;
use async_nats::jetstream::stream::Stream;

use sentinel_common::nats_config::{CONSUMER_NAME, STREAM_NAME, StreamConfig};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn connect_jetstream(url: &str) -> Result<jetstream::Context, BoxError> {
    let client = async_nats::connect(url).await?;
    Ok(jetstream::new(client))
}

pub async fn ensure_stream(
    js: &jetstream::Context,
    config: &StreamConfig,
) -> Result<Stream, BoxError> {
    let stream_config = jetstream::stream::Config {
        name: config.name.clone(),
        subjects: config.subjects.clone(),
        max_bytes: config.max_bytes,
        max_age: std::time::Duration::from_secs(config.max_age_secs),
        ..Default::default()
    };
    Ok(js.get_or_create_stream(stream_config).await?)
}

pub async fn create_pull_consumer(
    js: &jetstream::Context,
) -> Result<PullConsumer, BoxError> {
    let stream = js.get_stream(STREAM_NAME).await?;

    let consumer_config = jetstream::consumer::pull::Config {
        durable_name: Some(CONSUMER_NAME.into()),
        ack_policy: jetstream::consumer::AckPolicy::Explicit,
        max_deliver: 5,
        ..Default::default()
    };

    Ok(stream.get_or_create_consumer(CONSUMER_NAME, consumer_config).await?)
}
