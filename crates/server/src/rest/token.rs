use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::create_token;
use crate::rest::AppState;

const TOKEN_LIFETIME_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Deserialize)]
pub struct TokenRequest {
    secret: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    token: String,
    expires_in_secs: i64,
}

pub async fn create_api_token(
    State(state): State<AppState>,
    Json(body): Json<TokenRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    if body.secret.as_bytes() != state.jwt_secret.as_slice() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let expires_at_ms = now_ms + TOKEN_LIFETIME_MS;
    let token = create_token(&state.jwt_secret, "cli-admin", expires_at_ms);

    Ok(Json(TokenResponse {
        token,
        expires_in_secs: TOKEN_LIFETIME_MS / 1000,
    }))
}
