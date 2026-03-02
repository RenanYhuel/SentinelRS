use chrono::Utc;
use sentinel_common::proto::{
    server_message::Payload, HeartbeatPing, HeartbeatPong, ServerMessage,
};

use super::presence::{PresenceEvent, PresenceEventBus};
use super::registry::SessionRegistry;
use super::session::LiveSystemStats;

pub fn handle_heartbeat_ping(
    agent_id: &str,
    ping: &HeartbeatPing,
    registry: &SessionRegistry,
    events: &PresenceEventBus,
) -> ServerMessage {
    let now_ms = current_time_ms();
    let latency_ms = (now_ms - ping.timestamp_ms).max(0);

    let stats = ping
        .system_stats
        .as_ref()
        .map(proto_stats_to_live)
        .unwrap_or_default();

    let memory_percent = if stats.memory_total_bytes > 0 {
        (stats.memory_used_bytes as f64 / stats.memory_total_bytes as f64) * 100.0
    } else {
        0.0
    };

    registry.record_heartbeat(agent_id, latency_ms, stats);

    events.emit(PresenceEvent::HeartbeatReceived {
        agent_id: agent_id.to_string(),
        latency_ms,
        cpu_percent: ping
            .system_stats
            .as_ref()
            .map(|s| s.cpu_percent)
            .unwrap_or(0.0),
        memory_percent,
        at: Utc::now(),
    });

    let next_interval = compute_next_interval(latency_ms);

    ServerMessage {
        payload: Some(Payload::HeartbeatPong(HeartbeatPong {
            server_time_ms: now_ms,
            next_heartbeat_interval_ms: next_interval,
        })),
    }
}

fn proto_stats_to_live(s: &sentinel_common::proto::SystemStats) -> LiveSystemStats {
    LiveSystemStats {
        cpu_percent: s.cpu_percent,
        memory_used_bytes: s.memory_used_bytes,
        memory_total_bytes: s.memory_total_bytes,
        disk_used_bytes: s.disk_used_bytes,
        disk_total_bytes: s.disk_total_bytes,
        load_avg_1m: s.load_avg_1m,
        process_count: s.process_count,
        uptime_seconds: s.uptime_seconds,
        os_name: s.os_name.clone(),
        hostname: s.hostname.clone(),
    }
}

fn compute_next_interval(latency_ms: i64) -> i64 {
    if latency_ms > 2000 {
        15_000
    } else if latency_ms > 500 {
        12_000
    } else {
        0
    }
}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
