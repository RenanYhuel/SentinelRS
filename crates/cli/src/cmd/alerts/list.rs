use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct ListArgs {
    #[arg(long, help = "Filter by status (firing / resolved)")]
    pub status: Option<String>,

    #[arg(long, help = "Filter by agent ID")]
    pub agent: Option<String>,

    #[arg(long, help = "Filter by rule ID")]
    pub rule: Option<String>,

    #[arg(long, default_value = "50", help = "Max number of alerts")]
    pub limit: i64,
}

pub async fn run(args: ListArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching alerts...")),
        OutputMode::Json => None,
    };

    let mut query_parts = vec![format!("limit={}", args.limit)];
    if let Some(ref s) = args.status {
        query_parts.push(format!("status={s}"));
    }
    if let Some(ref a) = args.agent {
        query_parts.push(format!("agent_id={a}"));
    }
    if let Some(ref r) = args.rule {
        query_parts.push(format!("rule_id={r}"));
    }
    let qs = query_parts.join("&");
    let path = format!("/v1/alerts?{qs}");

    let alerts = api.get_json(&path).await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&alerts)?,
        OutputMode::Human => {
            let empty = vec![];
            let arr = alerts.as_array().unwrap_or(&empty);
            if arr.is_empty() {
                theme::print_dim("  No alerts found.");
                return Ok(());
            }
            theme::print_header("Alerts");
            let mut table = build_table(&[
                "Status", "Severity", "Rule", "Agent", "Metric", "Value", "Fired At",
            ]);
            for a in arr {
                table.add_row(vec![
                    a["status"].as_str().unwrap_or("-"),
                    a["severity"].as_str().unwrap_or("-"),
                    a["rule_name"].as_str().unwrap_or("-"),
                    a["agent_id"].as_str().unwrap_or("-"),
                    a["metric_name"].as_str().unwrap_or("-"),
                    &a["value"].to_string(),
                    a["fired_at"].as_str().unwrap_or("-"),
                ]);
            }
            println!("{table}");
        }
    }

    Ok(())
}
