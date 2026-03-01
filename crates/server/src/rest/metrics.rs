use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use std::sync::Arc;

use crate::metrics::exposition::render_prometheus;
use crate::metrics::server_metrics::ServerMetrics;

pub async fn metrics(State(m): State<Arc<ServerMetrics>>) -> impl IntoResponse {
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
        let m = ServerMetrics::new();
        m.inc_grpc_requests();
        let resp = metrics(State(m)).await.into_response();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("sentinel_server_grpc_requests_total 1"));
    }
}
