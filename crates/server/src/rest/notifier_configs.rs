use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::persistence::NotifierConfigRecord;
use crate::rest::AppState;

#[derive(Deserialize)]
pub struct CreateNotifierRequest {
    pub name: String,
    pub ntype: String,
    pub config: serde_json::Value,
    pub enabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateNotifierRequest {
    pub name: Option<String>,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

#[derive(Serialize)]
pub struct NotifierConfigResponse {
    pub id: String,
    pub name: String,
    pub ntype: String,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at_ms: i64,
}

fn to_response(r: NotifierConfigRecord) -> NotifierConfigResponse {
    NotifierConfigResponse {
        id: r.id,
        name: r.name,
        ntype: r.ntype,
        config: r.config,
        enabled: r.enabled,
        created_at_ms: r.created_at_ms,
    }
}

const VALID_TYPES: &[&str] = &[
    "webhook",
    "slack",
    "discord",
    "smtp",
    "telegram",
    "pagerduty",
    "teams",
    "opsgenie",
    "gotify",
    "ntfy",
];

pub async fn list_notifier_configs(
    State(state): State<AppState>,
) -> Result<Json<Vec<NotifierConfigResponse>>, StatusCode> {
    let repo = state
        .notifier_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let records = repo.list_all().await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, "notifier_configs list failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(records.into_iter().map(to_response).collect()))
}

pub async fn get_notifier_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<NotifierConfigResponse>, StatusCode> {
    let repo = state
        .notifier_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let record = repo.get(&id).await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, "notifier_configs get failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    record
        .map(|r| Json(to_response(r)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_notifier_config(
    State(state): State<AppState>,
    Json(body): Json<CreateNotifierRequest>,
) -> Result<(StatusCode, Json<NotifierConfigResponse>), StatusCode> {
    if !VALID_TYPES.contains(&body.ntype.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let repo = state
        .notifier_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let record = NotifierConfigRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: body.name,
        ntype: body.ntype,
        config: body.config,
        enabled: body.enabled.unwrap_or(true),
        created_at_ms: now_ms,
    };

    repo.insert(&record).await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, "notifier_configs insert failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::CREATED, Json(to_response(record))))
}

pub async fn update_notifier_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateNotifierRequest>,
) -> Result<Json<NotifierConfigResponse>, StatusCode> {
    let repo = state
        .notifier_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let existing = repo
        .get(&id)
        .await
        .map_err(|e| {
            tracing::error!(target: "rest", error = %e, "notifier_configs get failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let updated = NotifierConfigRecord {
        id: existing.id,
        name: body.name.unwrap_or(existing.name),
        ntype: existing.ntype,
        config: body.config.unwrap_or(existing.config),
        enabled: body.enabled.unwrap_or(existing.enabled),
        created_at_ms: existing.created_at_ms,
    };

    repo.update(&updated).await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, "notifier_configs update failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(to_response(updated)))
}

pub async fn toggle_notifier_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<NotifierConfigResponse>, StatusCode> {
    let repo = state
        .notifier_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let existing = repo
        .get(&id)
        .await
        .map_err(|e| {
            tracing::error!(target: "rest", error = %e, "notifier_configs get failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let toggled = NotifierConfigRecord {
        enabled: !existing.enabled,
        ..existing
    };

    repo.update(&toggled).await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, "notifier_configs toggle failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(to_response(toggled)))
}

pub async fn delete_notifier_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    let Some(repo) = state.notifier_repo.as_ref() else {
        return StatusCode::SERVICE_UNAVAILABLE;
    };
    match repo.delete(&id).await {
        Ok(true) => StatusCode::NO_CONTENT,
        Ok(false) => StatusCode::NOT_FOUND,
        Err(e) => {
            tracing::error!(target: "rest", error = %e, "notifier_configs delete failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
