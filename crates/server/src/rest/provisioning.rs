use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::provisioning::store::BootstrapToken;
use crate::provisioning::token::generate_bootstrap_token;
use crate::rest::AppState;

#[derive(Deserialize)]
pub struct GenerateInstallRequest {
    pub name: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default = "default_ttl_minutes")]
    pub ttl_minutes: i64,
}

fn default_ttl_minutes() -> i64 {
    30
}

#[derive(Serialize)]
pub struct GenerateInstallResponse {
    pub token: String,
    pub expires_at: String,
    pub install_command: String,
}

pub async fn generate_install(
    State(state): State<AppState>,
    Json(body): Json<GenerateInstallRequest>,
) -> Result<Json<GenerateInstallResponse>, StatusCode> {
    if body.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    if body.ttl_minutes < 1 || body.ttl_minutes > 1440 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let token_store = match &state.token_store {
        Some(ts) => ts,
        None => return Err(StatusCode::SERVICE_UNAVAILABLE),
    };

    let token = generate_bootstrap_token();
    let now = Utc::now();
    let expires_at = now + Duration::minutes(body.ttl_minutes);

    let entry = BootstrapToken {
        token: token.clone(),
        agent_name: body.name,
        labels: body.labels,
        created_by: "api".into(),
        created_at: now,
        expires_at,
        consumed: false,
    };

    token_store.insert(entry);

    tracing::info!(
        target: "auth",
        token = %sentinel_common::redact::RedactedSecret(&token),
        "provisioning token generated"
    );

    let server_url = &state.grpc_public_url;

    let install_command = format!(
        "docker run -d --name sentinel-agent \
         -v sentinel_data:/etc/sentinel \
         -e BOOTSTRAP_TOKEN={token} \
         -e SERVER_URL={server_url} \
         sentinelrs/agent:latest"
    );

    Ok(Json(GenerateInstallResponse {
        token,
        expires_at: expires_at.to_rfc3339(),
        install_command,
    }))
}
