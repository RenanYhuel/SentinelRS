use axum::Router;
use axum::routing::{get, post};

use crate::store::{AgentStore, RuleStore};
use super::{agents, health, notifiers, rules};

#[derive(Clone)]
pub struct AppState {
    pub agents: AgentStore,
    pub rules: RuleStore,
    pub jwt_secret: Vec<u8>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/ready", get(health::ready))
        .route("/v1/agents", get(agents::list_agents))
        .route("/v1/agents/{agent_id}", get(agents::get_agent))
        .route("/v1/rules", get(rules::list_rules).post(rules::create_rule))
        .route(
            "/v1/rules/{rule_id}",
            get(rules::get_rule)
                .put(rules::update_rule)
                .delete(rules::delete_rule),
        )
        .route("/v1/notifiers/test", post(notifiers::test_notifier))
        .with_state(state)
}
