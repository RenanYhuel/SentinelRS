use axum::Router;
use axum::routing::get;

use crate::store::AgentStore;
use super::agents;
use super::health;

#[derive(Clone)]
pub struct AppState {
    pub agents: AgentStore,
    pub jwt_secret: Vec<u8>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/ready", get(health::ready))
        .route("/v1/agents", get(agents::list_agents))
        .route("/v1/agents/{agent_id}", get(agents::get_agent))
        .with_state(state)
}
