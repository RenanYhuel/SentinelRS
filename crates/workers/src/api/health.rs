use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
}

pub async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
    })
}

pub async fn ready() -> StatusCode {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn healthz_ok() {
        let resp = healthz().await;
        assert_eq!(resp.0.status, "ok");
    }

    #[tokio::test]
    async fn ready_ok() {
        assert_eq!(ready().await, StatusCode::OK);
    }
}
