use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::rest::AppState;
use crate::store::rule_record::RuleRecord;

#[derive(Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub agent_pattern: Option<String>,
    pub metric_name: String,
    pub condition: String,
    pub threshold: f64,
    pub for_duration_ms: Option<i64>,
    pub severity: Option<String>,
    pub annotations: Option<HashMap<String, String>>,
    pub enabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateRuleRequest {
    pub name: Option<String>,
    pub agent_pattern: Option<String>,
    pub metric_name: Option<String>,
    pub condition: Option<String>,
    pub threshold: Option<f64>,
    pub for_duration_ms: Option<i64>,
    pub severity: Option<String>,
    pub annotations: Option<HashMap<String, String>>,
    pub enabled: Option<bool>,
}

#[derive(Serialize)]
pub struct RuleResponse {
    pub id: String,
    pub name: String,
    pub agent_pattern: String,
    pub metric_name: String,
    pub condition: String,
    pub threshold: f64,
    pub for_duration_ms: i64,
    pub severity: String,
    pub annotations: HashMap<String, String>,
    pub enabled: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

fn to_response(r: RuleRecord) -> RuleResponse {
    RuleResponse {
        id: r.id,
        name: r.name,
        agent_pattern: r.agent_pattern,
        metric_name: r.metric_name,
        condition: r.condition,
        threshold: r.threshold,
        for_duration_ms: r.for_duration_ms,
        severity: r.severity,
        annotations: r.annotations,
        enabled: r.enabled,
        created_at_ms: r.created_at_ms,
        updated_at_ms: r.updated_at_ms,
    }
}

fn validate_condition(c: &str) -> bool {
    matches!(
        c,
        "GreaterThan" | "LessThan" | "GreaterOrEqual" | "LessOrEqual" | "Equal"
    )
}

fn validate_severity(s: &str) -> bool {
    matches!(s, "info" | "warning" | "critical")
}

pub async fn list_rules(State(state): State<AppState>) -> Json<Vec<RuleResponse>> {
    let rules = state.rules.list().into_iter().map(to_response).collect();
    Json(rules)
}

pub async fn get_rule(
    State(state): State<AppState>,
    Path(rule_id): Path<String>,
) -> Result<Json<RuleResponse>, StatusCode> {
    state
        .rules
        .get(&rule_id)
        .map(|r| Json(to_response(r)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_rule(
    State(state): State<AppState>,
    Json(body): Json<CreateRuleRequest>,
) -> Result<(StatusCode, Json<RuleResponse>), StatusCode> {
    if !validate_condition(&body.condition) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let severity = body.severity.as_deref().unwrap_or("warning");
    if !validate_severity(severity) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let record = RuleRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: body.name,
        agent_pattern: body.agent_pattern.unwrap_or_else(|| "*".into()),
        metric_name: body.metric_name,
        condition: body.condition,
        threshold: body.threshold,
        for_duration_ms: body.for_duration_ms.unwrap_or(0),
        severity: severity.to_string(),
        annotations: body.annotations.unwrap_or_default(),
        enabled: body.enabled.unwrap_or(true),
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
    };

    let resp = to_response(record.clone());
    state.rules.insert(record);
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn update_rule(
    State(state): State<AppState>,
    Path(rule_id): Path<String>,
    Json(body): Json<UpdateRuleRequest>,
) -> Result<Json<RuleResponse>, StatusCode> {
    let existing = state.rules.get(&rule_id).ok_or(StatusCode::NOT_FOUND)?;

    if let Some(ref c) = body.condition {
        if !validate_condition(c) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if let Some(ref s) = body.severity {
        if !validate_severity(s) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let updated = RuleRecord {
        id: existing.id.clone(),
        name: body.name.unwrap_or(existing.name.clone()),
        agent_pattern: body.agent_pattern.unwrap_or(existing.agent_pattern.clone()),
        metric_name: body.metric_name.unwrap_or(existing.metric_name.clone()),
        condition: body.condition.unwrap_or(existing.condition.clone()),
        threshold: body.threshold.unwrap_or(existing.threshold),
        for_duration_ms: body.for_duration_ms.unwrap_or(existing.for_duration_ms),
        severity: body.severity.unwrap_or(existing.severity.clone()),
        annotations: body.annotations.unwrap_or(existing.annotations.clone()),
        enabled: body.enabled.unwrap_or(existing.enabled),
        created_at_ms: existing.created_at_ms,
        updated_at_ms: now_ms,
    };

    state.rules.update(updated.clone());
    Ok(Json(to_response(updated)))
}

pub async fn delete_rule(State(state): State<AppState>, Path(rule_id): Path<String>) -> StatusCode {
    if state.rules.delete(&rule_id) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
