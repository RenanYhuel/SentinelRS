use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use super::router::AppState;

pub async fn cluster_status(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.registry.cluster_stats();
    Json(stats)
}

pub async fn agent_live(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> impl IntoResponse {
    match state.registry.snapshot(&agent_id) {
        Some(snap) => axum::response::Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_string(&snap).unwrap_or_default(),
            ))
            .unwrap(),
        None => axum::response::Response::builder()
            .status(404)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(r#"{"error":"agent not connected"}"#))
            .unwrap(),
    }
}

pub async fn cluster_events(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let event_type = match &event {
                crate::stream::PresenceEvent::AgentConnected { .. } => "agent_connected",
                crate::stream::PresenceEvent::AgentDisconnected { .. } => "agent_disconnected",
                crate::stream::PresenceEvent::AgentStale { .. } => "agent_stale",
                crate::stream::PresenceEvent::HeartbeatReceived { .. } => "heartbeat",
            };
            let data = serde_json::to_string(&event).unwrap_or_default();
            Some(Ok(Event::default().event(event_type).data(data)))
        }
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

pub async fn agent_ids(State(state): State<AppState>) -> impl IntoResponse {
    let ids = state.registry.connected_agent_ids();
    Json(ids)
}
