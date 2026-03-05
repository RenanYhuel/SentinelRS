use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use std::sync::Arc;

use super::state::WorkerState;
use crate::metrics::exposition::render_prometheus;

pub async fn metrics(State(state): State<Arc<WorkerState>>) -> impl IntoResponse {
    let body = render_prometheus(&state.metrics, state.identity.id(), &state.pool);
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backpressure::{BatchSemaphore, CircuitBreaker};
    use crate::identity::WorkerIdentity;
    use crate::metrics::worker_metrics::WorkerMetrics;
    use axum::extract::State;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    #[tokio::test]
    async fn handler_returns_prometheus() {
        let m = WorkerMetrics::new();
        m.inc_batches_processed();

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://fake@localhost/fake")
            .unwrap();

        let state = Arc::new(WorkerState {
            identity: WorkerIdentity::generate(),
            metrics: m,
            circuit_breaker: CircuitBreaker::new(5, Duration::from_secs(30)),
            semaphore: Arc::new(BatchSemaphore::new(100)),
            in_flight: Arc::new(AtomicU64::new(0)),
            registry: None,
            pool,
        });

        let resp = metrics(State(state)).await.into_response();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("sentinel_worker_batches_processed_total"));
        assert!(text.contains("sentinel_worker_pool_size"));
    }
}
