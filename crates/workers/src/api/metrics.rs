use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use std::sync::Arc;

use crate::metrics::exposition::render_prometheus;
use crate::metrics::worker_metrics::WorkerMetrics;

pub async fn metrics(State(m): State<Arc<WorkerMetrics>>) -> impl IntoResponse {
    let body = render_prometheus(&m);
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;

    #[tokio::test]
    async fn handler_returns_prometheus() {
        let m = WorkerMetrics::new();
        m.inc_batches_processed();
        let resp = metrics(State(m)).await.into_response();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("sentinel_worker_batches_processed_total 1"));
    }
}
