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
