use sentinel_common::proto::{server_message::Payload, HeartbeatPong, ServerMessage};

use super::registry::SessionRegistry;

pub fn handle_heartbeat_ping(
    agent_id: &str,
    _timestamp_ms: i64,
    registry: &SessionRegistry,
) -> ServerMessage {
    registry.touch(agent_id);

    ServerMessage {
        payload: Some(Payload::HeartbeatPong(HeartbeatPong {
            server_time_ms: current_time_ms(),
        })),
    }
}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
