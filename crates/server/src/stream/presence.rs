use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

const DEFAULT_CHANNEL_CAPACITY: usize = 4096;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PresenceEvent {
    AgentConnected {
        agent_id: String,
        agent_version: String,
        #[serde(with = "chrono::serde::ts_milliseconds")]
        at: DateTime<Utc>,
    },
    AgentDisconnected {
        agent_id: String,
        reason: DisconnectReason,
        connected_duration_ms: i64,
        #[serde(with = "chrono::serde::ts_milliseconds")]
        at: DateTime<Utc>,
    },
    AgentStale {
        agent_id: String,
        last_ping_ms_ago: i64,
        expected_interval_ms: i64,
        #[serde(with = "chrono::serde::ts_milliseconds")]
        at: DateTime<Utc>,
    },
    HeartbeatReceived {
        agent_id: String,
        latency_ms: i64,
        cpu_percent: f64,
        memory_percent: f64,
        #[serde(with = "chrono::serde::ts_milliseconds")]
        at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisconnectReason {
    StreamClosed,
    StaleTimeout,
    Evicted,
    ServerShutdown,
}

#[derive(Clone)]
pub struct PresenceEventBus {
    tx: broadcast::Sender<PresenceEvent>,
    event_seq: Arc<AtomicU64>,
}

impl Default for PresenceEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl PresenceEventBus {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHANNEL_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            event_seq: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn emit(&self, event: PresenceEvent) {
        self.event_seq.fetch_add(1, Ordering::Relaxed);
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PresenceEvent> {
        self.tx.subscribe()
    }

    pub fn seq(&self) -> u64 {
        self.event_seq.load(Ordering::Relaxed)
    }

    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}
