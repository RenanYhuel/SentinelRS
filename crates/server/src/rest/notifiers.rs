use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::rest::AppState;

#[derive(Deserialize)]
pub struct TestNotifierRequest {
    pub notifier_type: String,
    pub config: serde_json::Value,
}

#[derive(Serialize)]
pub struct TestNotifierResponse {
    pub success: bool,
    pub message: String,
}

pub async fn test_notifier(
    State(_state): State<AppState>,
    Json(body): Json<TestNotifierRequest>,
) -> Result<Json<TestNotifierResponse>, StatusCode> {
    let result = match body.notifier_type.as_str() {
        "webhook" => validate_webhook(&body.config),
        "slack" => validate_slack(&body.config),
        "discord" => validate_discord(&body.config),
        "smtp" => validate_smtp(&body.config),
        _ => Err("unknown notifier type".into()),
    };

    match result {
        Ok(msg) => Ok(Json(TestNotifierResponse {
            success: true,
            message: msg,
        })),
        Err(msg) => Ok(Json(TestNotifierResponse {
            success: false,
            message: msg,
        })),
    }
}

fn validate_webhook(config: &serde_json::Value) -> Result<String, String> {
    let url = config
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("missing 'url' field")?;
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("url must start with http:// or https://".into());
    }
    Ok(format!("webhook config valid (url: {})", url))
}

fn validate_slack(config: &serde_json::Value) -> Result<String, String> {
    let url = config
        .get("webhook_url")
        .and_then(|v| v.as_str())
        .ok_or("missing 'webhook_url' field")?;
    if !url.contains("hooks.slack.com") {
        return Err("webhook_url must contain hooks.slack.com".into());
    }
    Ok("slack config valid".into())
}

fn validate_discord(config: &serde_json::Value) -> Result<String, String> {
    let url = config
        .get("webhook_url")
        .and_then(|v| v.as_str())
        .ok_or("missing 'webhook_url' field")?;
    if !url.contains("discord.com/api/webhooks") {
        return Err("webhook_url must contain discord.com/api/webhooks".into());
    }
    Ok("discord config valid".into())
}

fn validate_smtp(config: &serde_json::Value) -> Result<String, String> {
    config
        .get("host")
        .and_then(|v| v.as_str())
        .ok_or("missing 'host' field")?;
    config
        .get("from")
        .and_then(|v| v.as_str())
        .ok_or("missing 'from' field")?;
    config
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or("missing 'to' field")?;
    Ok("smtp config valid".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn webhook_valid() {
        let cfg = json!({"url": "https://example.com/hook"});
        assert!(validate_webhook(&cfg).is_ok());
    }

    #[test]
    fn webhook_missing_url() {
        let cfg = json!({});
        assert!(validate_webhook(&cfg).is_err());
    }

    #[test]
    fn webhook_bad_scheme() {
        let cfg = json!({"url": "ftp://example.com"});
        assert!(validate_webhook(&cfg).is_err());
    }

    #[test]
    fn slack_valid() {
        let cfg = json!({"webhook_url": "https://hooks.slack.com/services/xxx"});
        assert!(validate_slack(&cfg).is_ok());
    }

    #[test]
    fn slack_invalid() {
        let cfg = json!({"webhook_url": "https://example.com"});
        assert!(validate_slack(&cfg).is_err());
    }

    #[test]
    fn discord_valid() {
        let cfg = json!({"webhook_url": "https://discord.com/api/webhooks/123/abc"});
        assert!(validate_discord(&cfg).is_ok());
    }

    #[test]
    fn discord_invalid() {
        let cfg = json!({"webhook_url": "https://example.com"});
        assert!(validate_discord(&cfg).is_err());
    }

    #[test]
    fn smtp_valid() {
        let cfg = json!({"host": "smtp.example.com", "from": "a@b.com", "to": "c@d.com"});
        assert!(validate_smtp(&cfg).is_ok());
    }

    #[test]
    fn smtp_missing_host() {
        let cfg = json!({"from": "a@b.com", "to": "c@d.com"});
        assert!(validate_smtp(&cfg).is_err());
    }
}
