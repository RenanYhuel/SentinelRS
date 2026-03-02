use axum::extract::State;
use axum::Json;
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::backpressure::State as CbState;

use super::state::WorkerState;

#[derive(Serialize)]
pub struct StatusResponse {
    pub worker_id: String,
    pub hostname: String,
    pub started_at: String,
    pub uptime: String,
    pub uptime_secs: u64,
    pub circuit_breaker: CircuitBreakerStatus,
    pub pipeline: PipelineStatus,
    pub peers: Vec<String>,
}

#[derive(Serialize)]
pub struct CircuitBreakerStatus {
    pub state: String,
    pub failure_count: u32,
    pub total_trips: u64,
}

#[derive(Serialize)]
pub struct PipelineStatus {
    pub in_flight: u64,
    pub concurrent_batches_in_use: usize,
    pub concurrent_batches_max: usize,
    pub batches_processed: u64,
    pub batches_errors: u64,
    pub rows_inserted: u64,
    pub messages_acked: u64,
    pub messages_nacked: u64,
    pub alerts_fired: u64,
}

pub async fn status(State(state): State<Arc<WorkerState>>) -> Json<StatusResponse> {
    let cb_state = state.circuit_breaker.state().await;
    let cb_state_str = match cb_state {
        CbState::Closed => "closed",
        CbState::Open => "open",
        CbState::HalfOpen => "half-open",
    };

    let peers: Vec<String> = state
        .registry
        .as_ref()
        .map(|r| r.peers())
        .unwrap_or_default();

    let in_flight = state.in_flight.load(Ordering::Relaxed);

    Json(StatusResponse {
        worker_id: state.identity.id().to_string(),
        hostname: state.identity.hostname().to_string(),
        started_at: state.identity.started_at().to_string(),
        uptime: state.identity.uptime_human(),
        uptime_secs: state.identity.uptime_secs(),
        circuit_breaker: CircuitBreakerStatus {
            state: cb_state_str.into(),
            failure_count: state.circuit_breaker.failure_count(),
            total_trips: state.circuit_breaker.total_trips(),
        },
        pipeline: PipelineStatus {
            in_flight,
            concurrent_batches_in_use: state.semaphore.in_use(),
            concurrent_batches_max: state.semaphore.max(),
            batches_processed: state.metrics.batches_processed_val(),
            batches_errors: state.metrics.batches_errors_val(),
            rows_inserted: state.metrics.rows_inserted_val(),
            messages_acked: state.metrics.messages_acked_val(),
            messages_nacked: state.metrics.messages_nacked_val(),
            alerts_fired: state.metrics.alerts_fired_val(),
        },
        peers,
    })
}
