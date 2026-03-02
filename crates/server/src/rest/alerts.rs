use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::rest::AppState;

#[derive(Deserialize)]
pub struct AlertQuery {
    pub status: Option<String>,
    pub agent_id: Option<String>,
    pub rule_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AlertRow {
    pub id: String,
    pub fingerprint: String,
    pub rule_id: String,
    pub rule_name: String,
    pub agent_id: String,
    pub metric_name: String,
    pub severity: String,
    pub status: String,
    pub value: f64,
    pub threshold: f64,
    pub fired_at: chrono::DateTime<chrono::Utc>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub annotations: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_alerts(
    State(state): State<AppState>,
    Query(q): Query<AlertQuery>,
) -> Result<Json<Vec<AlertRow>>, StatusCode> {
    let pool = state.pool.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let limit = q.limit.unwrap_or(100).min(1000);

    let rows = match (&q.status, &q.agent_id, &q.rule_id) {
        (Some(status), _, _) => {
            sqlx::query_as::<_, AlertRow>(
                "SELECT id, fingerprint, rule_id, rule_name, agent_id, metric_name,
                        severity, status, value, threshold, fired_at, resolved_at,
                        annotations, created_at
                 FROM alerts WHERE status = $1
                 ORDER BY created_at DESC LIMIT $2",
            )
            .bind(status)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (_, Some(agent_id), _) => {
            sqlx::query_as::<_, AlertRow>(
                "SELECT id, fingerprint, rule_id, rule_name, agent_id, metric_name,
                        severity, status, value, threshold, fired_at, resolved_at,
                        annotations, created_at
                 FROM alerts WHERE agent_id = $1
                 ORDER BY created_at DESC LIMIT $2",
            )
            .bind(agent_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (_, _, Some(rule_id)) => {
            sqlx::query_as::<_, AlertRow>(
                "SELECT id, fingerprint, rule_id, rule_name, agent_id, metric_name,
                        severity, status, value, threshold, fired_at, resolved_at,
                        annotations, created_at
                 FROM alerts WHERE rule_id = $1
                 ORDER BY created_at DESC LIMIT $2",
            )
            .bind(rule_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        _ => {
            sqlx::query_as::<_, AlertRow>(
                "SELECT id, fingerprint, rule_id, rule_name, agent_id, metric_name,
                        severity, status, value, threshold, fired_at, resolved_at,
                        annotations, created_at
                 FROM alerts ORDER BY created_at DESC LIMIT $1",
            )
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    };

    rows.map(Json).map_err(|e| {
        tracing::error!(target: "rest", error = %e, "alert query failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

pub async fn get_alert(
    State(state): State<AppState>,
    Path(alert_id): Path<String>,
) -> Result<Json<AlertRow>, StatusCode> {
    let pool = state.pool.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    sqlx::query_as::<_, AlertRow>(
        "SELECT id, fingerprint, rule_id, rule_name, agent_id, metric_name,
                severity, status, value, threshold, fired_at, resolved_at,
                annotations, created_at
         FROM alerts WHERE id = $1",
    )
    .bind(&alert_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(target: "rest", error = %e, "alert query failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map(Json)
    .ok_or(StatusCode::NOT_FOUND)
}
