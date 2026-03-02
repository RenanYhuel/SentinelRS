use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::mpsc;

use sentinel_common::proto::ServerMessage;

use super::latency::{LatencySnapshot, LatencyTracker};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    PendingAuth,
    Authenticated,
    Closed,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LiveSystemStats {
    pub cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub load_avg_1m: f64,
    pub process_count: u32,
    pub uptime_seconds: u64,
    pub os_name: String,
    pub hostname: String,
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
    pub heartbeat_count: u64,
    pub system_stats: LiveSystemStats,
    pub latency: LatencyTracker,
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
            heartbeat_count: 0,
            system_stats: LiveSystemStats::default(),
            latency: LatencyTracker::new(),
            tx,
        }
    }

    pub fn touch(&mut self) {
        self.last_ping = Utc::now();
        self.heartbeat_count += 1;
    }

    pub fn record_latency(&mut self, latency_ms: i64) {
        self.latency.record(latency_ms);
    }

    pub fn update_system_stats(&mut self, stats: LiveSystemStats) {
        self.system_stats = stats;
    }

    pub fn is_stale(&self, timeout_ms: i64) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_ping)
            .num_milliseconds();
        elapsed > timeout_ms
    }

    pub fn ms_since_last_ping(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.last_ping)
            .num_milliseconds()
    }

    pub fn connection_duration_ms(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.connected_at)
            .num_milliseconds()
    }

    pub fn memory_percent(&self) -> f64 {
        if self.system_stats.memory_total_bytes == 0 {
            return 0.0;
        }
        (self.system_stats.memory_used_bytes as f64 / self.system_stats.memory_total_bytes as f64)
            * 100.0
    }

    pub fn disk_percent(&self) -> f64 {
        if self.system_stats.disk_total_bytes == 0 {
            return 0.0;
        }
        (self.system_stats.disk_used_bytes as f64 / self.system_stats.disk_total_bytes as f64)
            * 100.0
    }

    pub fn connection_quality(&self) -> ConnectionQuality {
        let latency = self.latency.snapshot();
        let jitter = latency.jitter_ms;
        let avg = latency.avg_ms;

        if avg < 50.0 && jitter < 10.0 {
            ConnectionQuality::Excellent
        } else if avg < 150.0 && jitter < 30.0 {
            ConnectionQuality::Good
        } else if avg < 500.0 && jitter < 100.0 {
            ConnectionQuality::Fair
        } else {
            ConnectionQuality::Poor
        }
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            agent_id: self.agent_id.clone(),
            agent_version: self.agent_version.clone(),
            capabilities: self.capabilities.clone(),
            state: self.state.clone(),
            connected_at: self.connected_at,
            last_ping: self.last_ping,
            heartbeat_interval_ms: self.heartbeat_interval_ms,
            heartbeat_count: self.heartbeat_count,
            connection_duration_ms: self.connection_duration_ms(),
            system_stats: self.system_stats.clone(),
            latency: self.latency.snapshot(),
            connection_quality: self.connection_quality(),
            memory_percent: self.memory_percent(),
            disk_percent: self.disk_percent(),
        }
    }

    pub async fn send(&self, msg: ServerMessage) -> Result<(), SendError> {
        self.tx
            .send(Ok(msg))
            .await
            .map_err(|_| SendError::ChannelClosed)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionQuality {
    Excellent,
    Good,
    Fair,
    Poor,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSnapshot {
    pub agent_id: String,
    pub agent_version: String,
    pub capabilities: Vec<String>,
    pub state: SessionState,
    pub connected_at: DateTime<Utc>,
    pub last_ping: DateTime<Utc>,
    pub heartbeat_interval_ms: i64,
    pub heartbeat_count: u64,
    pub connection_duration_ms: i64,
    pub system_stats: LiveSystemStats,
    pub latency: LatencySnapshot,
    pub connection_quality: ConnectionQuality,
    pub memory_percent: f64,
    pub disk_percent: f64,
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
