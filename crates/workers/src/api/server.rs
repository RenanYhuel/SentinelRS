use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;

use super::{health, metrics};
use crate::metrics::worker_metrics::WorkerMetrics;

pub fn router(worker_metrics: Arc<WorkerMetrics>) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/ready", get(health::ready))
        .route("/metrics", get(metrics::metrics).with_state(worker_metrics))
}

pub async fn serve(
    listener: TcpListener,
    worker_metrics: Arc<WorkerMetrics>,
) -> std::io::Result<()> {
    let app = router(worker_metrics);
    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

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
        let m = WorkerMetrics::new();
        let app = router(m);

        let (status, _) = send(app.clone(), "/healthz").await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = send(app.clone(), "/ready").await;
        assert_eq!(status, StatusCode::OK);

        let (status, body) = send(app, "/metrics").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("sentinel_worker_"));
    }
}
