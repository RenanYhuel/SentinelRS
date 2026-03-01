use async_nats::jetstream;
use async_nats::jetstream::stream::Stream;

use sentinel_common::nats_config::StreamConfig;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn ensure_stream(
    js: &jetstream::Context,
    config: &StreamConfig,
) -> Result<Stream, BoxError> {
    let retention = match config.retention {
        sentinel_common::nats_config::RetentionPolicy::Limits => {
            async_nats::jetstream::stream::RetentionPolicy::Limits
        }
        sentinel_common::nats_config::RetentionPolicy::WorkQueue => {
            async_nats::jetstream::stream::RetentionPolicy::WorkQueue
        }
    };

    let storage = match config.storage {
        sentinel_common::nats_config::StorageType::File => {
            async_nats::jetstream::stream::StorageType::File
        }
        sentinel_common::nats_config::StorageType::Memory => {
            async_nats::jetstream::stream::StorageType::Memory
        }
    };

    let stream_config = jetstream::stream::Config {
        name: config.name.clone(),
        subjects: config.subjects.clone(),
        max_bytes: config.max_bytes,
        max_age: std::time::Duration::from_secs(config.max_age_secs),
        retention,
        storage,
        num_replicas: config.num_replicas,
        ..Default::default()
    };

    Ok(js.get_or_create_stream(stream_config).await?)
}

pub async fn connect_jetstream(url: &str) -> Result<jetstream::Context, BoxError> {
    let client = async_nats::connect(url).await?;
    Ok(jetstream::new(client))
}
