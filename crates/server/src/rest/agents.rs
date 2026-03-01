use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::rest::AppState;

#[derive(Serialize)]
pub struct AgentSummary {
    pub agent_id: String,
    pub hw_id: String,
    pub agent_version: String,
    pub registered_at_ms: i64,
}

pub async fn list_agents(State(state): State<AppState>) -> Json<Vec<AgentSummary>> {
    let agents = state
        .agents
        .list()
        .into_iter()
        .map(|r| AgentSummary {
            agent_id: r.agent_id,
            hw_id: r.hw_id,
            agent_version: r.agent_version,
            registered_at_ms: r.registered_at_ms,
        })
        .collect();
    Json(agents)
}

pub async fn get_agent(
    State(state): State<AppState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> Result<Json<AgentSummary>, StatusCode> {
    state
        .agents
        .get(&agent_id)
        .map(|r| {
            Json(AgentSummary {
                agent_id: r.agent_id,
                hw_id: r.hw_id,
                agent_version: r.agent_version,
                registered_at_ms: r.registered_at_ms,
            })
        })
        .ok_or(StatusCode::NOT_FOUND)
}
