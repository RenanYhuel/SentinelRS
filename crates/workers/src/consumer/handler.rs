use async_nats::jetstream::consumer::PullConsumer;
use async_nats::jetstream::Message;
use futures::StreamExt;
use prost::Message as ProstMessage;

use sentinel_common::proto::Batch;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub enum HandleError {
    Decode(String),
    Processing(String),
}

impl std::fmt::Display for HandleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decode(e) => write!(f, "decode: {e}"),
            Self::Processing(e) => write!(f, "processing: {e}"),
        }
    }
}

pub fn decode_batch(msg: &Message) -> Result<Batch, HandleError> {
    Batch::decode(msg.payload.as_ref()).map_err(|e| HandleError::Decode(e.to_string()))
}

pub fn extract_header(msg: &Message, key: &str) -> Option<String> {
    msg.headers
        .as_ref()?
        .get(key)
        .map(|v| v.to_string())
}

pub async fn pull_batch(
    consumer: &PullConsumer,
    max_messages: usize,
) -> Result<Vec<Message>, BoxError> {
    let mut messages = consumer.fetch().max_messages(max_messages).messages().await?;
    let mut batch = Vec::with_capacity(max_messages);
    while let Some(Ok(msg)) = messages.next().await {
        batch.push(msg);
    }
    Ok(batch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message as ProstMessage;
    use sentinel_common::proto::Batch;

    #[test]
    fn decode_valid_batch() {
        let batch = Batch {
            batch_id: "b-1".into(),
            agent_id: "agent-1".into(),
            ..Default::default()
        };
        let encoded = batch.encode_to_vec();
        let decoded = Batch::decode(encoded.as_slice()).unwrap();
        assert_eq!(decoded.batch_id, "b-1");
        assert_eq!(decoded.agent_id, "agent-1");
    }

    #[test]
    fn decode_invalid_bytes() {
        let result = Batch::decode(&[0xFF, 0xFF][..]);
        assert!(result.is_err());
    }

    #[test]
    fn error_display() {
        let e = HandleError::Decode("bad proto".into());
        assert!(e.to_string().contains("decode"));
    }
}
