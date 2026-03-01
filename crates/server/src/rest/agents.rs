use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::agent_queries;
use crate::rest::AppState;

#[derive(Serialize)]
pub struct AgentSummary {
    pub agent_id: String,
    pub hw_id: String,
    pub agent_version: String,
    pub registered_at_ms: i64,
    pub last_seen: Option<DateTime<Utc>>,
}

pub async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<Vec<AgentSummary>>, StatusCode> {
    if let Some(ref pool) = state.pool {
        let rows = agent_queries::fetch_all(pool).await.map_err(|e| {
            tracing::error!(error = %e, "failed to fetch agents from database");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        let agents = rows
            .into_iter()
            .map(|r| AgentSummary {
                agent_id: r.agent_id,
                hw_id: r.hw_id,
                agent_version: r.agent_version,
                registered_at_ms: r.registered_at_ms,
                last_seen: r.last_seen,
            })
            .collect();
        return Ok(Json(agents));
    }

    let agents = state
        .agents
        .list()
        .into_iter()
        .map(|r| AgentSummary {
            agent_id: r.agent_id,
            hw_id: r.hw_id,
            agent_version: r.agent_version,
            registered_at_ms: r.registered_at_ms,
            last_seen: r.last_seen,
        })
        .collect();
    Ok(Json(agents))
}

pub async fn get_agent(
    State(state): State<AppState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> Result<Json<AgentSummary>, StatusCode> {
    if let Some(ref pool) = state.pool {
        let row = agent_queries::fetch_one(pool, &agent_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, %agent_id, "failed to fetch agent from database");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;
        return Ok(Json(AgentSummary {
            agent_id: row.agent_id,
            hw_id: row.hw_id,
            agent_version: row.agent_version,
            registered_at_ms: row.registered_at_ms,
            last_seen: row.last_seen,
        }));
    }

    state
        .agents
        .get(&agent_id)
        .map(|r| {
            Json(AgentSummary {
                agent_id: r.agent_id,
                hw_id: r.hw_id,
                agent_version: r.agent_version,
                registered_at_ms: r.registered_at_ms,
                last_seen: r.last_seen,
            })
        })
        .ok_or(StatusCode::NOT_FOUND)
}
