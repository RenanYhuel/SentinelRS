use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{input, print_json, select, spinner, theme, OutputMode};

const CONDITIONS: &[&str] = &[
    "GreaterThan",
    "LessThan",
    "GreaterOrEqual",
    "LessOrEqual",
    "Equal",
];

const SEVERITIES: &[&str] = &["info", "warning", "critical"];

#[derive(Args)]
pub struct CreateArgs {
    #[arg(long, help = "JSON file path or inline JSON (skip for interactive)")]
    pub data: Option<String>,
}

pub async fn run(args: CreateArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let body = match args.data {
        Some(ref raw) => parse_json_data(raw)?,
        None => build_interactive()?,
    };

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Creating rule...")),
        OutputMode::Json => None,
    };

    let created = api.post_json("/v1/rules", &body).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Rule created");
    }

    match mode {
        OutputMode::Json => print_json(&created)?,
        OutputMode::Human => {
            theme::print_kv("ID", created["id"].as_str().unwrap_or("-"));
        }
    }

    Ok(())
}

fn build_interactive() -> Result<serde_json::Value> {
    let name = input::text_required("Rule name")?;
    let metric_name = input::text_required("Metric name")?;
    let cond_idx = select::select_option("Condition", CONDITIONS).unwrap_or(0);
    let threshold = input::text("Threshold", "80.0")?;
    let sev_idx = select::select_option("Severity", SEVERITIES).unwrap_or(1);

    Ok(serde_json::json!({
        "name": name,
        "metric_name": metric_name,
        "condition": CONDITIONS[cond_idx],
        "threshold": threshold.parse::<f64>().unwrap_or(80.0),
        "severity": SEVERITIES[sev_idx],
    }))
}

fn parse_json_data(raw: &str) -> Result<serde_json::Value> {
    if std::path::Path::new(raw).exists() {
        let content = std::fs::read_to_string(raw)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(serde_json::from_str(raw)?)
    }
}
