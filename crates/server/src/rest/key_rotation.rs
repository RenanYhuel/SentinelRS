use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;

use crate::rest::AppState;

#[derive(Serialize)]
pub struct RotateKeyResponse {
    pub agent_id: String,
    pub new_key_id: String,
    pub new_secret: String,
}

pub async fn rotate_key(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<RotateKeyResponse>, StatusCode> {
    let (new_key_id, new_secret) = state
        .agents
        .rotate_key(&agent_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(RotateKeyResponse {
        agent_id,
        new_key_id,
        new_secret: STANDARD.encode(&new_secret),
    }))
}
