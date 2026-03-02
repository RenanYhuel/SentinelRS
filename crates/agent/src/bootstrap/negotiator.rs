use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::transport::Channel;

use sentinel_common::proto::sentinel_stream_client::SentinelStreamClient;
use sentinel_common::proto::{
    agent_message::Payload as AgentPayload, server_message::Payload as ServerPayload, AgentMessage,
    BootstrapRequest, BootstrapStatus,
};

const OUTBOUND_BUFFER: usize = 16;
const BOOTSTRAP_TIMEOUT_SECS: u64 = 30;

pub struct BootstrapResult {
    pub agent_id: String,
    pub config_yaml: Vec<u8>,
}

pub async fn negotiate(
    server_url: &str,
    bootstrap_token: &str,
    hw_id: &str,
) -> Result<BootstrapResult, NegotiateError> {
    let channel = Channel::from_shared(server_url.to_string())
        .map_err(|e| NegotiateError::Config(e.to_string()))?
        .connect()
        .await
        .map_err(|e| NegotiateError::Transport(e.to_string()))?;

    let mut client = SentinelStreamClient::new(channel);

    let (tx, rx) = mpsc::channel::<AgentMessage>(OUTBOUND_BUFFER);
    let outbound = ReceiverStream::new(rx);

    let bootstrap_msg = AgentMessage {
        payload: Some(AgentPayload::BootstrapRequest(BootstrapRequest {
            bootstrap_token: bootstrap_token.into(),
            hw_id: hw_id.into(),
            agent_version: env!("CARGO_PKG_VERSION").into(),
        })),
    };

    tx.send(bootstrap_msg)
        .await
        .map_err(|_| NegotiateError::ChannelClosed)?;

    let response = client
        .open_stream(outbound)
        .await
        .map_err(|e| NegotiateError::Rpc(e.to_string()))?;

    let mut inbound = response.into_inner();

    let msg = tokio::time::timeout(
        std::time::Duration::from_secs(BOOTSTRAP_TIMEOUT_SECS),
        inbound.next(),
    )
    .await
    .map_err(|_| NegotiateError::Timeout)?
    .ok_or(NegotiateError::StreamClosed)?
    .map_err(|e| NegotiateError::Transport(e.to_string()))?;

    match msg.payload {
        Some(ServerPayload::BootstrapResponse(resp)) => {
            let status = BootstrapStatus::try_from(resp.status)
                .unwrap_or(BootstrapStatus::BootstrapInvalidToken);

            match status {
                BootstrapStatus::BootstrapOk => Ok(BootstrapResult {
                    agent_id: resp.agent_id,
                    config_yaml: resp.config_yaml,
                }),
                BootstrapStatus::BootstrapInvalidToken => {
                    Err(NegotiateError::Rejected(resp.message))
                }
                BootstrapStatus::BootstrapExpiredToken => {
                    Err(NegotiateError::Expired(resp.message))
                }
            }
        }
        Some(ServerPayload::Error(err)) => Err(NegotiateError::ServerError(err.message)),
        _ => Err(NegotiateError::UnexpectedResponse),
    }
}

#[derive(Debug)]
pub enum NegotiateError {
    Config(String),
    Transport(String),
    ChannelClosed,
    Rpc(String),
    Timeout,
    StreamClosed,
    Rejected(String),
    Expired(String),
    ServerError(String),
    UnexpectedResponse,
}

impl std::fmt::Display for NegotiateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(e) => write!(f, "config: {e}"),
            Self::Transport(e) => write!(f, "transport: {e}"),
            Self::ChannelClosed => write!(f, "channel closed"),
            Self::Rpc(e) => write!(f, "rpc: {e}"),
            Self::Timeout => write!(f, "bootstrap timeout"),
            Self::StreamClosed => write!(f, "stream closed"),
            Self::Rejected(m) => write!(f, "rejected: {m}"),
            Self::Expired(m) => write!(f, "expired: {m}"),
            Self::ServerError(m) => write!(f, "server error: {m}"),
            Self::UnexpectedResponse => write!(f, "unexpected server response"),
        }
    }
}

impl std::error::Error for NegotiateError {}
