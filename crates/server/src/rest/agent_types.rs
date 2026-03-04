use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::stream::registry::SessionRegistry;
use crate::stream::session::SessionSnapshot;

use super::agent_queries::AgentSummaryRow;

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Online,
    Offline,
    Stale,
    Bootstrapping,
}

#[derive(Serialize)]
pub struct AgentSummary {
    pub agent_id: String,
    pub hw_id: String,
    pub agent_version: String,
    pub registered_at_ms: i64,
    pub last_seen: Option<DateTime<Utc>>,
    pub status: AgentStatus,
    pub connected_since: Option<DateTime<Utc>>,
    pub connection_duration_ms: Option<i64>,
    pub latency_ms: Option<f64>,
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub disk_percent: Option<f64>,
    pub uptime_seconds: Option<u64>,
    pub os_name: Option<String>,
    pub hostname: Option<String>,
    pub connection_quality: Option<String>,
    pub heartbeat_count: Option<u64>,
}

#[derive(Serialize)]
pub struct AgentHealth {
    pub agent_id: String,
    pub status: AgentStatus,
    pub connected_since: Option<DateTime<Utc>>,
    pub connection_duration_ms: Option<i64>,
    pub connection_quality: Option<String>,
    pub capabilities: Vec<String>,
    pub heartbeat_count: u64,
    pub heartbeat_interval_ms: i64,
    pub latency: Option<LatencyDetail>,
    pub system: Option<SystemDetail>,
}

#[derive(Serialize)]
pub struct LatencyDetail {
    pub avg_ms: f64,
    pub min_ms: i64,
    pub max_ms: i64,
    pub p50_ms: i64,
    pub p95_ms: i64,
    pub p99_ms: i64,
    pub jitter_ms: f64,
    pub sample_count: u64,
}

#[derive(Serialize)]
pub struct SystemDetail {
    pub cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_percent: f64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_percent: f64,
    pub load_avg_1m: f64,
    pub process_count: u32,
    pub uptime_seconds: u64,
    pub os_name: String,
    pub hostname: String,
}

#[derive(Serialize)]
pub struct FleetOverview {
    pub total: usize,
    pub online: usize,
    pub offline: usize,
    pub stale: usize,
    pub bootstrapping: usize,
    pub avg_cpu_percent: f64,
    pub avg_memory_percent: f64,
    pub avg_latency_ms: f64,
    pub agents: Vec<AgentSummary>,
}

pub fn resolve_status(
    registry: &SessionRegistry,
    agent_id: &str,
    last_seen: Option<DateTime<Utc>>,
) -> (AgentStatus, Option<SessionSnapshot>) {
    match registry.snapshot(agent_id) {
        Some(snap) => {
            let elapsed = Utc::now()
                .signed_duration_since(snap.last_ping)
                .num_milliseconds();
            let threshold = snap.heartbeat_interval_ms * 3;
            let status = if elapsed > threshold {
                AgentStatus::Stale
            } else {
                AgentStatus::Online
            };
            (status, Some(snap))
        }
        None => {
            let status = if last_seen.is_none() {
                AgentStatus::Bootstrapping
            } else {
                AgentStatus::Offline
            };
            (status, None)
        }
    }
}

pub fn enrich(row: AgentSummaryRow, registry: &SessionRegistry) -> AgentSummary {
    let (status, snap) = resolve_status(registry, &row.agent_id, row.last_seen);
    build_summary(row, status, snap.as_ref())
}

fn build_summary(
    row: AgentSummaryRow,
    status: AgentStatus,
    snap: Option<&SessionSnapshot>,
) -> AgentSummary {
    AgentSummary {
        agent_id: row.agent_id,
        hw_id: row.hw_id,
        agent_version: row.agent_version,
        registered_at_ms: row.registered_at_ms,
        last_seen: row.last_seen,
        status,
        connected_since: snap.map(|s| s.connected_at),
        connection_duration_ms: snap.map(|s| s.connection_duration_ms),
        latency_ms: snap.map(|s| s.latency.avg_ms),
        cpu_percent: snap.map(|s| s.system_stats.cpu_percent),
        memory_percent: snap.map(|s| s.memory_percent),
        disk_percent: snap.map(|s| s.disk_percent),
        uptime_seconds: snap.map(|s| s.system_stats.uptime_seconds),
        os_name: snap.map(|s| s.system_stats.os_name.clone()),
        hostname: snap.map(|s| s.system_stats.hostname.clone()),
        connection_quality: snap.map(|s| format!("{:?}", s.connection_quality).to_lowercase()),
        heartbeat_count: snap.map(|s| s.heartbeat_count),
    }
}

pub fn build_health(agent_id: &str, snap: &SessionSnapshot) -> AgentHealth {
    AgentHealth {
        agent_id: agent_id.to_string(),
        status: AgentStatus::Online,
        connected_since: Some(snap.connected_at),
        connection_duration_ms: Some(snap.connection_duration_ms),
        connection_quality: Some(format!("{:?}", snap.connection_quality).to_lowercase()),
        capabilities: snap.capabilities.clone(),
        heartbeat_count: snap.heartbeat_count,
        heartbeat_interval_ms: snap.heartbeat_interval_ms,
        latency: Some(LatencyDetail {
            avg_ms: snap.latency.avg_ms,
            min_ms: snap.latency.min_ms,
            max_ms: snap.latency.max_ms,
            p50_ms: snap.latency.p50_ms,
            p95_ms: snap.latency.p95_ms,
            p99_ms: snap.latency.p99_ms,
            jitter_ms: snap.latency.jitter_ms,
            sample_count: snap.latency.sample_count,
        }),
        system: Some(SystemDetail {
            cpu_percent: snap.system_stats.cpu_percent,
            memory_used_bytes: snap.system_stats.memory_used_bytes,
            memory_total_bytes: snap.system_stats.memory_total_bytes,
            memory_percent: snap.memory_percent,
            disk_used_bytes: snap.system_stats.disk_used_bytes,
            disk_total_bytes: snap.system_stats.disk_total_bytes,
            disk_percent: snap.disk_percent,
            load_avg_1m: snap.system_stats.load_avg_1m,
            process_count: snap.system_stats.process_count,
            uptime_seconds: snap.system_stats.uptime_seconds,
            os_name: snap.system_stats.os_name.clone(),
            hostname: snap.system_stats.hostname.clone(),
        }),
    }
}

pub fn build_fleet_overview(agents: Vec<AgentSummary>) -> FleetOverview {
    let total = agents.len();
    let online = agents
        .iter()
        .filter(|a| a.status == AgentStatus::Online)
        .count();
    let offline = agents
        .iter()
        .filter(|a| a.status == AgentStatus::Offline)
        .count();
    let stale = agents
        .iter()
        .filter(|a| a.status == AgentStatus::Stale)
        .count();
    let bootstrapping = agents
        .iter()
        .filter(|a| a.status == AgentStatus::Bootstrapping)
        .count();

    let online_agents: Vec<&AgentSummary> = agents
        .iter()
        .filter(|a| a.status == AgentStatus::Online)
        .collect();

    let n = online_agents.len().max(1) as f64;
    let avg_cpu = online_agents
        .iter()
        .filter_map(|a| a.cpu_percent)
        .sum::<f64>()
        / n;
    let avg_mem = online_agents
        .iter()
        .filter_map(|a| a.memory_percent)
        .sum::<f64>()
        / n;
    let avg_lat = online_agents
        .iter()
        .filter_map(|a| a.latency_ms)
        .sum::<f64>()
        / n;

    FleetOverview {
        total,
        online,
        offline,
        stale,
        bootstrapping,
        avg_cpu_percent: avg_cpu,
        avg_memory_percent: avg_mem,
        avg_latency_ms: avg_lat,
        agents,
    }
}
