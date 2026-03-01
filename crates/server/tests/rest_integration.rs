use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use sentinel_server::rest::{router, AppState};
use sentinel_server::store::{AgentRecord, AgentStore, RuleStore};

fn app_state() -> AppState {
    AppState {
        agents: AgentStore::new(),
        rules: RuleStore::new(),
        jwt_secret: b"test-secret".to_vec(),
    }
}

fn app() -> axum::Router {
    router(app_state())
}

fn seed_agent(state: &AppState) {
    state.agents.insert(AgentRecord {
        agent_id: "agent-1".into(),
        hw_id: "hw-abc".into(),
        secret: b"secret".to_vec(),
        key_id: "key-1".into(),
        agent_version: "0.1.0".into(),
        registered_at_ms: 1_700_000_000_000,
        deprecated_keys: Vec::new(),
    });
}

#[tokio::test]
async fn healthz_returns_ok() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn ready_returns_ok() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn list_agents_empty() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/v1/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let agents: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(agents.is_empty());
}

#[tokio::test]
async fn list_agents_with_seeded_data() {
    let state = app_state();
    seed_agent(&state);
    let app = router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let agents: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["agent_id"], "agent-1");
}

#[tokio::test]
async fn get_agent_found() {
    let state = app_state();
    seed_agent(&state);
    let app = router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/agents/agent-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let agent: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(agent["hw_id"], "hw-abc");
}

#[tokio::test]
async fn get_agent_not_found() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/v1/agents/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_rule_and_list() {
    let state = app_state();
    let app = router(state.clone());

    let body = serde_json::json!({
        "name": "high-cpu",
        "metric_name": "cpu.usage",
        "condition": "GreaterThan",
        "threshold": 90.0,
        "severity": "critical"
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/rules")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);

    let app2 = router(state);
    let resp2 = app2
        .oneshot(
            Request::builder()
                .uri("/v1/rules")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(resp2.into_body(), usize::MAX)
        .await
        .unwrap();
    let rules: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["name"], "high-cpu");
    assert_eq!(rules[0]["condition"], "GreaterThan");
}

#[tokio::test]
async fn create_rule_invalid_condition() {
    let body = serde_json::json!({
        "name": "bad-rule",
        "metric_name": "cpu.usage",
        "condition": "InvalidOp",
        "threshold": 90.0
    });

    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/rules")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_rule_not_found() {
    let resp = app()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/v1/rules/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_notifier_webhook_valid() {
    let body = serde_json::json!({
        "notifier_type": "webhook",
        "config": { "url": "https://example.com/hook" }
    });

    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/notifiers/test")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["success"], true);
}

#[tokio::test]
async fn test_notifier_unknown_type() {
    let body = serde_json::json!({
        "notifier_type": "carrier_pigeon",
        "config": {}
    });

    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/notifiers/test")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["success"], false);
}

#[tokio::test]
async fn update_rule() {
    let state = app_state();

    let create_body = serde_json::json!({
        "name": "test-rule",
        "metric_name": "mem.used",
        "condition": "GreaterThan",
        "threshold": 80.0
    });

    let resp = router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/rules")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let rule_id = created["id"].as_str().unwrap();

    let update_body = serde_json::json!({
        "threshold": 95.0,
        "name": "updated-rule"
    });

    let resp2 = router(state.clone())
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/v1/rules/{rule_id}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp2.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp2.into_body(), usize::MAX)
        .await
        .unwrap();
    let updated: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated["name"], "updated-rule");
    assert_eq!(updated["threshold"], 95.0);
}
