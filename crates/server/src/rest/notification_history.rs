use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::rest::AppState;

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub notifier_id: Option<String>,
    pub alert_id: Option<String>,
}

#[derive(Serialize)]
pub struct HistoryEntryResponse {
    pub id: String,
    pub alert_id: String,
    pub notifier_id: String,
    pub ntype: String,
    pub status: String,
    pub error: Option<String>,
    pub attempts: i32,
    pub duration_ms: i32,
    pub sent_at_ms: i64,
}

#[derive(Serialize)]
pub struct HistoryStatsResponse {
    pub total: i64,
    pub sent: i64,
    pub failed: i64,
    pub avg_duration_ms: i64,
}

pub async fn list_notification_history(
    State(state): State<AppState>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<Vec<HistoryEntryResponse>>, StatusCode> {
    let repo = state
        .history_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let limit = q.limit.unwrap_or(50).min(500);
    let offset = q.offset.unwrap_or(0);

    let records = if let Some(ref nid) = q.notifier_id {
        repo.list_by_notifier(nid, limit).await
    } else if let Some(ref aid) = q.alert_id {
        repo.list_by_alert(aid, limit).await
    } else {
        repo.list(limit, offset).await
    };

    let records = records.map_err(|e| {
        tracing::error!(target: "rest", error = %e, "notification_history list failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(records.into_iter().map(to_response).collect()))
}

pub async fn notification_stats(
    State(state): State<AppState>,
) -> Result<Json<HistoryStatsResponse>, StatusCode> {
    let repo = state
        .history_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let stats = repo.stats().await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, "notification_history stats failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(HistoryStatsResponse {
        total: stats.total,
        sent: stats.sent,
        failed: stats.failed,
        avg_duration_ms: stats.avg_duration_ms,
    }))
}

fn to_response(r: crate::persistence::NotificationHistoryRecord) -> HistoryEntryResponse {
    HistoryEntryResponse {
        id: r.id,
        alert_id: r.alert_id,
        notifier_id: r.notifier_id,
        ntype: r.ntype,
        status: r.status,
        error: r.error,
        attempts: r.attempts,
        duration_ms: r.duration_ms,
        sent_at_ms: r.sent_at_ms,
    }
}
