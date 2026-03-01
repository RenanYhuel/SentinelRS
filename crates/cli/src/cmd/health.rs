use anyhow::{Context, Result};
use clap::Args;

use crate::output::{OutputMode, print_json, print_success, print_error};
use super::helpers;

#[derive(Args)]
pub struct HealthArgs;

pub async fn execute(
    _args: HealthArgs,
    mode: OutputMode,
    server: Option<String>,
    config_path: Option<String>,
) -> Result<()> {
    let base = helpers::resolve_rest_url(server.as_deref(), config_path.as_deref())?;

    let healthz = check_endpoint(&format!("{base}/healthz")).await;
    let ready = check_endpoint(&format!("{base}/ready")).await;

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "healthz": healthz.is_ok(),
            "ready": ready.is_ok(),
        }))?,
        OutputMode::Human => {
            match &healthz {
                Ok(_) => print_success("Health check: OK"),
                Err(e) => print_error(&format!("Health check: {e}")),
            }
            match &ready {
                Ok(_) => print_success("Ready check: OK"),
                Err(e) => print_error(&format!("Ready check: {e}")),
            }
        }
    }

    Ok(())
}

async fn check_endpoint(url: &str) -> Result<()> {
    let resp = reqwest::get(url)
        .await
        .context("connection failed")?;

    if resp.status().is_success() {
        Ok(())
    } else {
        anyhow::bail!("HTTP {}", resp.status())
    }
}
