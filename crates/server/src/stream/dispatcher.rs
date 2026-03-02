use std::sync::Arc;

use sentinel_common::proto::{
    agent_message::Payload as AgentPayload, server_message::Payload as ServerPayload, AgentMessage,
    ServerError, ServerMessage,
};

use crate::broker::BrokerPublisher;
use crate::metrics::server_metrics::ServerMetrics;
use crate::store::{AgentStore, IdempotencyStore};

use super::heartbeat_handler::handle_heartbeat_ping;
use super::metrics_handler::{handle_metrics_batch, MetricsHandlerCtx};
use super::presence::PresenceEventBus;
use super::registry::SessionRegistry;

#[allow(clippy::too_many_arguments)]
pub async fn dispatch(
    agent_id: &str,
    key_id: &str,
    msg: AgentMessage,
    agents: &AgentStore,
    idempotency: &IdempotencyStore,
    broker: &dyn BrokerPublisher,
    registry: &SessionRegistry,
    events: &PresenceEventBus,
    grace_period_ms: i64,
    metrics: &Arc<ServerMetrics>,
) -> Option<ServerMessage> {
    let payload = match msg.payload {
        Some(p) => p,
        None => return Some(error_message(400, "empty message payload", false)),
    };

    match payload {
        AgentPayload::MetricsBatch(batch) => {
            let ctx = MetricsHandlerCtx {
                agents,
                idempotency,
                broker,
                grace_period_ms,
                metrics,
            };
            let response = handle_metrics_batch(agent_id, key_id, batch, &ctx).await;
            Some(response)
        }
        AgentPayload::HeartbeatPing(ping) => {
            let response = handle_heartbeat_ping(agent_id, &ping, registry, events);
            Some(response)
        }
        AgentPayload::Handshake(_) => Some(error_message(
            400,
            "unexpected handshake on authenticated stream",
            false,
        )),
        AgentPayload::BootstrapRequest(_) => Some(error_message(
            400,
            "unexpected bootstrap on authenticated stream",
            false,
        )),
    }
}

fn error_message(code: u32, message: &str, fatal: bool) -> ServerMessage {
    ServerMessage {
        payload: Some(ServerPayload::Error(ServerError {
            code,
            message: message.into(),
            fatal,
        })),
    }
}
