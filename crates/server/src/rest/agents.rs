use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use super::agent_queries;
use super::agent_types::{self, AgentSummary};
use crate::rest::AppState;

pub async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<Vec<AgentSummary>>, StatusCode> {
    if let Some(ref pool) = state.pool {
        let rows = agent_queries::fetch_all(pool).await.map_err(|e| {
            tracing::error!(target: "rest", error = %e, "failed to fetch agents — check database connectivity");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        let agents = rows
            .into_iter()
            .map(|r| agent_types::enrich(r, &state.registry))
            .collect();
        return Ok(Json(agents));
    }

    let agents = state
        .agents
        .list()
        .into_iter()
        .map(|r| {
            let row = agent_queries::AgentSummaryRow {
                agent_id: r.agent_id,
                hw_id: r.hw_id,
                agent_version: r.agent_version,
                registered_at_ms: r.registered_at_ms,
                last_seen: r.last_seen,
            };
            agent_types::enrich(row, &state.registry)
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
                tracing::error!(target: "rest", error = %e, %agent_id, "failed to fetch agent — check database connectivity");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;
        return Ok(Json(agent_types::enrich(row, &state.registry)));
    }

    state
        .agents
        .get(&agent_id)
        .map(|r| {
            let row = agent_queries::AgentSummaryRow {
                agent_id: r.agent_id,
                hw_id: r.hw_id,
                agent_version: r.agent_version,
                registered_at_ms: r.registered_at_ms,
                last_seen: r.last_seen,
            };
            Json(agent_types::enrich(row, &state.registry))
        })
        .ok_or(StatusCode::NOT_FOUND)
}
