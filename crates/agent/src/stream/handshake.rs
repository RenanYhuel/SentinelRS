use sentinel_common::proto::{
    agent_message::Payload as AgentPayload, server_message::Payload as ServerPayload, AgentMessage,
    HandshakeRequest, HandshakeStatus, ServerMessage,
};

use crate::security::HmacSigner;

pub struct HandshakeParams {
    pub agent_id: String,
    pub agent_version: String,
    pub capabilities: Vec<String>,
    pub key_id: String,
}

pub fn build_handshake_message(params: &HandshakeParams, signer: &HmacSigner) -> AgentMessage {
    let timestamp_ms = current_time_ms();
    let canonical = format!("{}:{}:{}", params.agent_id, timestamp_ms, params.key_id);
    let signature = signer.sign_base64(canonical.as_bytes());

    AgentMessage {
        payload: Some(AgentPayload::Handshake(HandshakeRequest {
            agent_id: params.agent_id.clone(),
            agent_version: params.agent_version.clone(),
            capabilities: params.capabilities.clone(),
            key_id: params.key_id.clone(),
            timestamp_ms,
            signature,
        })),
    }
}

pub fn validate_handshake_ack(msg: &ServerMessage) -> Result<i64, HandshakeError> {
    let ack = match &msg.payload {
        Some(ServerPayload::HandshakeAck(ack)) => ack,
        _ => return Err(HandshakeError::UnexpectedResponse),
    };

    let status =
        HandshakeStatus::try_from(ack.status).unwrap_or(HandshakeStatus::HandshakeRejected);

    match status {
        HandshakeStatus::HandshakeOk => Ok(ack.heartbeat_interval_ms),
        HandshakeStatus::HandshakeRejected => Err(HandshakeError::Rejected(ack.message.clone())),
        HandshakeStatus::HandshakeUpgradeRequired => {
            Err(HandshakeError::UpgradeRequired(ack.message.clone()))
        }
    }
}

#[derive(Debug)]
pub enum HandshakeError {
    UnexpectedResponse,
    Rejected(String),
    UpgradeRequired(String),
}

impl std::fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedResponse => write!(f, "unexpected handshake response"),
            Self::Rejected(msg) => write!(f, "handshake rejected: {msg}"),
            Self::UpgradeRequired(msg) => write!(f, "upgrade required: {msg}"),
        }
    }
}

impl std::error::Error for HandshakeError {}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
