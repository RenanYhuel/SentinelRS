use async_nats::jetstream;
use async_nats::jetstream::consumer::PullConsumer;
use async_nats::jetstream::stream::Stream;

use sentinel_common::nats_config::{StreamConfig, STREAM_NAME};

use crate::config::ConsumerGroupConfig;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn connect_jetstream(
    url: &str,
) -> Result<(jetstream::Context, async_nats::Client), BoxError> {
    let client = async_nats::connect(url).await?;
    let js = jetstream::new(client.clone());
    Ok((js, client))
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

pub async fn create_group_consumer(
    js: &jetstream::Context,
    group_config: &ConsumerGroupConfig,
) -> Result<PullConsumer, BoxError> {
    let stream = js.get_stream(STREAM_NAME).await?;

    let consumer_config = jetstream::consumer::pull::Config {
        durable_name: Some(group_config.group_name.clone()),
        ack_policy: jetstream::consumer::AckPolicy::Explicit,
        max_deliver: group_config.max_deliver,
        max_ack_pending: group_config.max_ack_pending,
        ack_wait: group_config.ack_wait,
        ..Default::default()
    };

    Ok(stream
        .get_or_create_consumer(&group_config.group_name, consumer_config)
        .await?)
}
