use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;

use super::state::WorkerState;
use super::{health, metrics, status};

pub fn router(state: Arc<WorkerState>) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route(
            "/ready",
            get(health::ready).with_state(Arc::clone(&state.circuit_breaker)),
        )
        .route(
            "/metrics",
            get(metrics::metrics).with_state(Arc::clone(&state.metrics)),
        )
        .route("/status", get(status::status).with_state(state))
}

pub async fn serve(listener: TcpListener, state: Arc<WorkerState>) -> std::io::Result<()> {
    let app = router(state);
    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backpressure::{BatchSemaphore, CircuitBreaker};
    use crate::identity::WorkerIdentity;
    use crate::metrics::worker_metrics::WorkerMetrics;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;
    use tower::ServiceExt;

    fn test_state() -> Arc<WorkerState> {
        Arc::new(WorkerState {
            identity: WorkerIdentity::generate(),
            metrics: WorkerMetrics::new(),
            circuit_breaker: CircuitBreaker::new(5, Duration::from_secs(30)),
            semaphore: Arc::new(BatchSemaphore::new(100)),
            in_flight: Arc::new(AtomicU64::new(0)),
            registry: None,
        })
    }

    async fn send(app: Router, uri: &str) -> (StatusCode, String) {
        let req = Request::get(uri).body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let status = resp.status();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn routes_respond() {
        let state = test_state();
        let app = router(state);

        let (status, _) = send(app.clone(), "/healthz").await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = send(app.clone(), "/ready").await;
        assert_eq!(status, StatusCode::OK);

        let (status, body) = send(app.clone(), "/metrics").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("sentinel_worker_"));

        let (status, body) = send(app, "/status").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("worker_id"));
        assert!(body.contains("circuit_breaker"));
        assert!(body.contains("pipeline"));
    }
}
