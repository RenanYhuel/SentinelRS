use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use super::agent_queries;
use super::agent_types::{self, FleetOverview};
use crate::rest::AppState;

pub async fn fleet_overview(
    State(state): State<AppState>,
) -> Result<Json<FleetOverview>, StatusCode> {
    if let Some(ref pool) = state.pool {
        let rows = agent_queries::fetch_all(pool).await.map_err(|e| {
            tracing::error!(target: "rest", error = %e, "failed to fetch agents for fleet overview — check database connectivity");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        let agents: Vec<_> = rows
            .into_iter()
            .map(|r| agent_types::enrich(r, &state.registry))
            .collect();
        return Ok(Json(agent_types::build_fleet_overview(agents)));
    }

    let agents: Vec<_> = state
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
    Ok(Json(agent_types::build_fleet_overview(agents)))
}
