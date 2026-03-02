use sentinel_common::proto::{HandshakeAck, HandshakeRequest, HandshakeStatus};

use crate::auth::verify_signature;
use crate::store::AgentStore;

const REPLAY_WINDOW_MS: i64 = 5 * 60 * 1000;

pub struct AuthResult {
    pub agent_id: String,
    pub agent_version: String,
    pub capabilities: Vec<String>,
    pub key_id: String,
}

pub enum AuthOutcome {
    Authenticated(AuthResult),
    Rejected(HandshakeAck),
}

pub fn authenticate_handshake(
    agents: &AgentStore,
    req: &HandshakeRequest,
    grace_period_ms: i64,
) -> AuthOutcome {
    if req.agent_id.is_empty() {
        return AuthOutcome::Rejected(reject_ack("agent_id is required"));
    }

    let record = match agents.get(&req.agent_id) {
        Some(r) => r,
        None => return AuthOutcome::Rejected(reject_ack("unknown agent")),
    };

    let now_ms = current_time_ms();
    if req.timestamp_ms > 0 {
        let drift = (now_ms - req.timestamp_ms).abs();
        if drift > REPLAY_WINDOW_MS {
            return AuthOutcome::Rejected(reject_ack("handshake timestamp outside replay window"));
        }
    }

    let key_id = if req.key_id.is_empty() {
        None
    } else {
        Some(req.key_id.as_str())
    };

    let secret = match agents.find_key_secret(&req.agent_id, key_id, grace_period_ms) {
        Some(s) => s,
        None => return AuthOutcome::Rejected(reject_ack("unknown or expired key")),
    };

    let canonical = build_canonical_handshake(&req.agent_id, req.timestamp_ms, &req.key_id);
    if !verify_signature(&secret, canonical.as_bytes(), &req.signature) {
        return AuthOutcome::Rejected(reject_ack("invalid handshake signature"));
    }

    let effective_key_id = if req.key_id.is_empty() {
        record.key_id.clone()
    } else {
        req.key_id.clone()
    };

    AuthOutcome::Authenticated(AuthResult {
        agent_id: req.agent_id.clone(),
        agent_version: req.agent_version.clone(),
        capabilities: req.capabilities.clone(),
        key_id: effective_key_id,
    })
}

fn build_canonical_handshake(agent_id: &str, timestamp_ms: i64, key_id: &str) -> String {
    format!("{agent_id}:{timestamp_ms}:{key_id}")
}

fn reject_ack(message: &str) -> HandshakeAck {
    HandshakeAck {
        status: HandshakeStatus::HandshakeRejected.into(),
        message: message.into(),
        server_time_ms: current_time_ms(),
        heartbeat_interval_ms: 0,
    }
}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
