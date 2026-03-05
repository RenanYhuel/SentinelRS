use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;

use super::{
    agent_health, agent_metrics, agents, alerts, cluster, fleet, health, key_rotation, metrics,
    notification_history, notifier_configs, notifiers, provisioning, rules, token,
};
use crate::metrics::server_metrics::ServerMetrics;
use crate::middleware::require_auth;
use crate::persistence::{MetricsQueryRepo, NotificationHistoryRepo, NotifierRepo, RuleRepo};
use crate::provisioning::TokenStore;
use crate::store::{AgentStore, RuleStore};
use crate::stream::{PresenceEventBus, SessionRegistry};

#[derive(Clone)]
pub struct AppState {
    pub agents: AgentStore,
    pub rules: RuleStore,
    pub rule_repo: Option<Arc<RuleRepo>>,
    pub notifier_repo: Option<Arc<NotifierRepo>>,
    pub history_repo: Option<Arc<NotificationHistoryRepo>>,
    pub metrics_repo: Option<Arc<MetricsQueryRepo>>,
    pub jwt_secret: Vec<u8>,
    pub metrics: Arc<ServerMetrics>,
    pub pool: Option<PgPool>,
    pub token_store: Option<TokenStore>,
    pub grpc_public_url: String,
    pub registry: SessionRegistry,
    pub events: PresenceEventBus,
}

pub fn router(state: AppState) -> Router {
    let metrics_state = state.metrics.clone();
    let jwt_secret = state.jwt_secret.clone();

    let public = Router::new()
        .route("/healthz", get(health::healthz))
        .route("/ready", get(health::ready))
        .route("/metrics", get(metrics::metrics).with_state(metrics_state))
        .route(
            "/v1/agents/generate-install",
            post(provisioning::generate_install),
        )
        .route("/v1/auth/token", post(token::create_api_token));

    let protected = Router::new()
        .route("/v1/agents", get(agents::list_agents))
        .route("/v1/agents/:agent_id", get(agents::get_agent))
        .route(
            "/v1/agents/:agent_id/health",
            get(agent_health::agent_health),
        )
        .route(
            "/v1/agents/:agent_id/rotate-key",
            post(key_rotation::rotate_key),
        )
        .route("/v1/fleet/overview", get(fleet::fleet_overview))
        .route("/v1/rules", get(rules::list_rules).post(rules::create_rule))
        .route(
            "/v1/rules/:rule_id",
            get(rules::get_rule)
                .put(rules::update_rule)
                .delete(rules::delete_rule),
        )
        .route("/v1/alerts", get(alerts::list_alerts))
        .route("/v1/alerts/:alert_id", get(alerts::get_alert))
        .route("/v1/notifiers/test", post(notifiers::test_notifier))
        .route(
            "/v1/notifiers",
            get(notifier_configs::list_notifier_configs)
                .post(notifier_configs::create_notifier_config),
        )
        .route(
            "/v1/notifiers/:notifier_id",
            get(notifier_configs::get_notifier_config)
                .put(notifier_configs::update_notifier_config)
                .delete(notifier_configs::delete_notifier_config),
        )
        .route(
            "/v1/notifiers/:notifier_id/toggle",
            post(notifier_configs::toggle_notifier_config),
        )
        .route(
            "/v1/notifications/history",
            get(notification_history::list_notification_history),
        )
        .route(
            "/v1/notifications/stats",
            get(notification_history::notification_stats),
        )
        .route(
            "/v1/metrics/agents/:agent_id/latest",
            get(agent_metrics::latest_metrics),
        )
        .route(
            "/v1/metrics/agents/:agent_id/history",
            get(agent_metrics::metric_history),
        )
        .route(
            "/v1/metrics/agents/:agent_id/names",
            get(agent_metrics::metric_names),
        )
        .route(
            "/v1/metrics/agents/:agent_id/aggregates",
            get(agent_metrics::metric_aggregates),
        )
        .route(
            "/v1/metrics/agents/:agent_id/percentiles",
            get(agent_metrics::metric_percentiles),
        )
        .route(
            "/v1/metrics/agents/:agent_id/top",
            get(agent_metrics::top_metrics),
        )
        .route(
            "/v1/metrics/agents/:agent_id/export",
            get(agent_metrics::export_metrics),
        )
        .route("/v1/metrics/summary", get(agent_metrics::fleet_summary))
        .route("/v1/metrics/compare", get(agent_metrics::compare_agents))
        .route("/v1/metrics/heatmap", get(agent_metrics::heatmap))
        .route("/v1/cluster/status", get(cluster::cluster_status))
        .route("/v1/cluster/agents", get(cluster::agent_ids))
        .route("/v1/cluster/events", get(cluster::cluster_events))
        .route("/v1/agents/:agent_id/live", get(cluster::agent_live))
        .layer(middleware::from_fn(move |req, next| {
            let auth_fn = require_auth(jwt_secret.clone());
            auth_fn(req, next)
        }));

    public.merge(protected).with_state(state)
}
