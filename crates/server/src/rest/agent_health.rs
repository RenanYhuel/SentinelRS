use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use super::agent_types::{self, AgentHealth};
use crate::rest::AppState;

pub async fn agent_health(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<AgentHealth>, StatusCode> {
    let snap = state
        .registry
        .snapshot(&agent_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(agent_types::build_health(&agent_id, &snap)))
}
