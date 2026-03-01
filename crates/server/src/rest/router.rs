use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

use super::{agents, health, key_rotation, metrics, notifiers, rules};
use crate::metrics::server_metrics::ServerMetrics;
use crate::store::{AgentStore, RuleStore};

#[derive(Clone)]
pub struct AppState {
    pub agents: AgentStore,
    pub rules: RuleStore,
    pub jwt_secret: Vec<u8>,
    pub metrics: Arc<ServerMetrics>,
}

pub fn router(state: AppState) -> Router {
    let metrics_state = state.metrics.clone();
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/ready", get(health::ready))
        .route("/metrics", get(metrics::metrics).with_state(metrics_state))
        .route("/v1/agents", get(agents::list_agents))
        .route("/v1/agents/:agent_id", get(agents::get_agent))
        .route(
            "/v1/agents/:agent_id/rotate-key",
            post(key_rotation::rotate_key),
        )
        .route("/v1/rules", get(rules::list_rules).post(rules::create_rule))
        .route(
            "/v1/rules/:rule_id",
            get(rules::get_rule)
                .put(rules::update_rule)
                .delete(rules::delete_rule),
        )
        .route("/v1/notifiers/test", post(notifiers::test_notifier))
        .with_state(state)
}
