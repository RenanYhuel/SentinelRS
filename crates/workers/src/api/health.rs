use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use crate::backpressure::CircuitBreaker;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
}

pub async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
    })
}

pub async fn ready(State(cb): State<Arc<CircuitBreaker>>) -> StatusCode {
    if cb.allow().await {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn healthz_ok() {
        let resp = healthz().await;
        assert_eq!(resp.0.status, "ok");
    }

    #[tokio::test]
    async fn ready_ok_when_closed() {
        let cb = CircuitBreaker::new(5, Duration::from_secs(30));
        let resp = ready(State(cb)).await;
        assert_eq!(resp, StatusCode::OK);
    }

    #[tokio::test]
    async fn ready_unavailable_when_open() {
        let cb = CircuitBreaker::new(1, Duration::from_secs(60));
        cb.record_failure().await;
        let resp = ready(State(cb)).await;
        assert_eq!(resp, StatusCode::SERVICE_UNAVAILABLE);
    }
}
