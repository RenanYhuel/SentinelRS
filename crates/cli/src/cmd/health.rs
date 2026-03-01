use anyhow::{Context, Result};
use clap::Args;

use crate::output::{OutputMode, print_json, spinner, theme};
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

    if mode == OutputMode::Human {
        theme::print_header("Health Checks");
    }

    let sp_health = match mode {
        OutputMode::Human => Some(spinner::create("Checking /healthz ...")),
        OutputMode::Json => None,
    };

    let healthz = check_endpoint(&format!("{base}/healthz")).await;

    if let Some(sp) = sp_health {
        match &healthz {
            Ok(_) => spinner::finish_ok(&sp, "Health check: OK"),
            Err(e) => spinner::finish_err(&sp, &format!("Health check: {e}")),
        }
    }

    let sp_ready = match mode {
        OutputMode::Human => Some(spinner::create("Checking /ready ...")),
        OutputMode::Json => None,
    };

    let ready = check_endpoint(&format!("{base}/ready")).await;

    if let Some(sp) = sp_ready {
        match &ready {
            Ok(_) => spinner::finish_ok(&sp, "Ready check: OK"),
            Err(e) => spinner::finish_err(&sp, &format!("Ready check: {e}")),
        }
    }

    if mode == OutputMode::Json {
        print_json(&serde_json::json!({
            "healthz": healthz.is_ok(),
            "ready": ready.is_ok(),
        }))?;
    } else {
        println!();
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
