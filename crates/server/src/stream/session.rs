use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::mpsc;

use sentinel_common::proto::ServerMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    PendingAuth,
    Authenticated,
    Closed,
}

pub struct Session {
    pub agent_id: String,
    pub agent_version: String,
    pub capabilities: Vec<String>,
    pub key_id: String,
    pub state: SessionState,
    pub connected_at: DateTime<Utc>,
    pub last_ping: DateTime<Utc>,
    pub heartbeat_interval_ms: i64,
    pub tx: Arc<mpsc::Sender<Result<ServerMessage, tonic::Status>>>,
}

impl Session {
    pub fn new(
        agent_id: String,
        agent_version: String,
        capabilities: Vec<String>,
        key_id: String,
        heartbeat_interval_ms: i64,
        tx: Arc<mpsc::Sender<Result<ServerMessage, tonic::Status>>>,
    ) -> Self {
        let now = Utc::now();
        Self {
            agent_id,
            agent_version,
            capabilities,
            key_id,
            state: SessionState::Authenticated,
            connected_at: now,
            last_ping: now,
            heartbeat_interval_ms,
            tx,
        }
    }

    pub fn touch(&mut self) {
        self.last_ping = Utc::now();
    }

    pub fn is_stale(&self, timeout_ms: i64) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_ping)
            .num_milliseconds();
        elapsed > timeout_ms
    }

    pub async fn send(&self, msg: ServerMessage) -> Result<(), SendError> {
        self.tx
            .send(Ok(msg))
            .await
            .map_err(|_| SendError::ChannelClosed)
    }
}

#[derive(Debug)]
pub enum SendError {
    ChannelClosed,
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChannelClosed => write!(f, "session channel closed"),
        }
    }
}

impl std::error::Error for SendError {}
