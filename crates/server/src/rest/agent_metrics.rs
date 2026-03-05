use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

use crate::rest::AppState;

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub metric: String,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    #[serde(default = "default_interval")]
    pub interval: String,
}

fn default_interval() -> String {
    "5m".into()
}

#[derive(Deserialize)]
pub struct AggregateQuery {
    pub metric: String,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    #[serde(default = "default_granularity")]
    pub granularity: String,
}

fn default_granularity() -> String {
    "1h".into()
}

#[derive(Deserialize)]
pub struct CompareQuery {
    pub agents: String,
    pub metric: String,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    #[serde(default = "default_interval")]
    pub interval: String,
}

#[derive(Deserialize)]
pub struct PercentileQuery {
    pub metric: String,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct ExportQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub metric: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".into()
}

#[derive(Deserialize)]
pub struct TopQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Deserialize)]
pub struct HeatmapQuery {
    pub metric: String,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    #[serde(default = "default_interval")]
    pub interval: String,
}

fn default_range(
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Utc::now();
    let f = from.unwrap_or_else(|| now - Duration::hours(1));
    let t = to.unwrap_or(now);
    (f, t)
}

pub async fn latest_metrics(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = state
        .metrics_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let rows = repo.latest(&agent_id).await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, agent_id = %agent_id, "latest_metrics query failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({
        "agent_id": agent_id,
        "metrics": rows,
    })))
}

pub async fn metric_history(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = state
        .metrics_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let (from, to) = default_range(q.from, q.to);
    let points = repo
        .history(&agent_id, &q.metric, from, to, &q.interval)
        .await
        .map_err(|e| {
            tracing::error!(target: "rest", error = %e, agent_id = %agent_id, "metric_history query failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({
        "agent_id": agent_id,
        "metric": q.metric,
        "from": from,
        "to": to,
        "interval": q.interval,
        "points": points,
    })))
}

pub async fn metric_names(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = state
        .metrics_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let names = repo.metric_names(&agent_id).await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, agent_id = %agent_id, "metric_names query failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({
        "agent_id": agent_id,
        "names": names,
    })))
}

pub async fn fleet_summary(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = state
        .metrics_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let rows = repo.summary().await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, "fleet_summary query failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "agents": rows })))
}

pub async fn metric_aggregates(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Query(q): Query<AggregateQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = state
        .metrics_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let (from, to) = default_range(q.from, q.to);

    let points = match q.granularity.as_str() {
        "5m" => repo.history_5m(&agent_id, &q.metric, from, to).await,
        _ => repo.history_1h(&agent_id, &q.metric, from, to).await,
    }
    .map_err(|e| {
        tracing::error!(target: "rest", error = %e, agent_id = %agent_id, "metric_aggregates query failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "agent_id": agent_id,
        "metric": q.metric,
        "granularity": q.granularity,
        "points": points,
    })))
}

pub async fn compare_agents(
    State(state): State<AppState>,
    Query(q): Query<CompareQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = state
        .metrics_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let (from, to) = default_range(q.from, q.to);
    let agent_ids: Vec<String> = q.agents.split(',').map(|s| s.trim().to_string()).collect();

    let points = repo
        .compare(&agent_ids, &q.metric, from, to, &q.interval)
        .await
        .map_err(|e| {
            tracing::error!(target: "rest", error = %e, "compare_agents query failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "agents": agent_ids,
        "metric": q.metric,
        "from": from,
        "to": to,
        "points": points,
    })))
}

pub async fn metric_percentiles(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Query(q): Query<PercentileQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = state
        .metrics_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let (from, to) = default_range(q.from, q.to);

    let result = repo
        .percentiles(&agent_id, &q.metric, from, to)
        .await
        .map_err(|e| {
            tracing::error!(target: "rest", error = %e, agent_id = %agent_id, "metric_percentiles query failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "agent_id": agent_id,
        "metric": q.metric,
        "from": from,
        "to": to,
        "percentiles": result,
    })))
}

pub async fn top_metrics(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Query(q): Query<TopQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = state
        .metrics_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let rows = repo.top(&agent_id, q.limit).await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, agent_id = %agent_id, "top_metrics query failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({
        "agent_id": agent_id,
        "top": rows,
    })))
}

pub async fn export_metrics(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Query(q): Query<ExportQuery>,
) -> Result<axum::response::Response, StatusCode> {
    let repo = state
        .metrics_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let (from, to) = default_range(q.from, q.to);

    let rows = repo
        .export_raw(&agent_id, from, to, q.metric.as_deref())
        .await
        .map_err(|e| {
            tracing::error!(target: "rest", error = %e, agent_id = %agent_id, "export_metrics query failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match q.format.as_str() {
        "csv" => {
            let mut csv = String::from("time,agent_id,name,value\n");
            for r in &rows {
                csv.push_str(&format!(
                    "{},{},{},{}\n",
                    r.time.to_rfc3339(),
                    r.agent_id,
                    r.name,
                    r.value.map(|v| v.to_string()).unwrap_or_default()
                ));
            }
            Ok(axum::response::Response::builder()
                .header("content-type", "text/csv")
                .header(
                    "content-disposition",
                    format!("attachment; filename=\"{agent_id}-metrics.csv\""),
                )
                .body(axum::body::Body::from(csv))
                .unwrap())
        }
        _ => {
            let json = serde_json::json!({
                "agent_id": agent_id,
                "from": from,
                "to": to,
                "rows": rows,
            });
            Ok(Json(json).into_response())
        }
    }
}

pub async fn heatmap(
    State(state): State<AppState>,
    Query(q): Query<HeatmapQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = state
        .metrics_repo
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let (from, to) = default_range(q.from, q.to);

    let summary = repo.summary().await.map_err(|e| {
        tracing::error!(target: "rest", error = %e, "heatmap summary query failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let agent_ids: Vec<String> = summary.iter().map(|s| s.agent_id.clone()).collect();
    if agent_ids.is_empty() {
        return Ok(Json(serde_json::json!({ "agents": [], "points": [] })));
    }

    let points = repo
        .compare(&agent_ids, &q.metric, from, to, &q.interval)
        .await
        .map_err(|e| {
            tracing::error!(target: "rest", error = %e, "heatmap compare query failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "metric": q.metric,
        "agents": agent_ids,
        "from": from,
        "to": to,
        "points": points,
    })))
}

use axum::response::IntoResponse;
