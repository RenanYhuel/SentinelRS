use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct HistoryArgs {
    #[arg(long, help = "Filter by notifier ID")]
    pub notifier_id: Option<String>,

    #[arg(long, help = "Filter by alert ID")]
    pub alert_id: Option<String>,

    #[arg(long, default_value = "25", help = "Number of entries")]
    pub limit: i64,

    #[arg(long, help = "Show stats summary")]
    pub stats: bool,
}

pub async fn run(args: HistoryArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    if args.stats {
        return run_stats(&api, mode).await;
    }

    let mut url = format!("/v1/notifications/history?limit={}", args.limit);
    if let Some(ref nid) = args.notifier_id {
        url.push_str(&format!("&notifier_id={nid}"));
    }
    if let Some(ref aid) = args.alert_id {
        url.push_str(&format!("&alert_id={aid}"));
    }

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching notification history...")),
        OutputMode::Json => None,
    };

    let entries: serde_json::Value = api.get_json(&url).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "History loaded");
    }

    match mode {
        OutputMode::Json => print_json(&entries)?,
        OutputMode::Human => {
            let arr = entries.as_array().map(|a| a.as_slice()).unwrap_or(&[]);
            if arr.is_empty() {
                theme::print_dim("No notification history found");
                return Ok(());
            }

            theme::print_table_header(&[
                "ID", "Alert", "Notifier", "Type", "Status", "Attempts", "Duration",
            ]);
            for e in arr {
                theme::print_table_row(&[
                    &truncate(e["id"].as_str().unwrap_or("-"), 8),
                    &truncate(e["alert_id"].as_str().unwrap_or("-"), 8),
                    &truncate(e["notifier_id"].as_str().unwrap_or("-"), 8),
                    e["ntype"].as_str().unwrap_or("-"),
                    e["status"].as_str().unwrap_or("-"),
                    &e["attempts"].as_i64().unwrap_or(0).to_string(),
                    &format!("{}ms", e["duration_ms"].as_i64().unwrap_or(0)),
                ]);
            }
        }
    }

    Ok(())
}

async fn run_stats(api: &crate::client::ApiClient, mode: OutputMode) -> Result<()> {
    let stats: serde_json::Value = api.get_json("/v1/notifications/stats").await?;

    match mode {
        OutputMode::Json => print_json(&stats)?,
        OutputMode::Human => {
            theme::print_kv("Total", &stats["total"].as_i64().unwrap_or(0).to_string());
            theme::print_kv("Sent", &stats["sent"].as_i64().unwrap_or(0).to_string());
            theme::print_kv("Failed", &stats["failed"].as_i64().unwrap_or(0).to_string());
            theme::print_kv(
                "Avg duration",
                &format!("{}ms", stats["avg_duration_ms"].as_i64().unwrap_or(0)),
            );
        }
    }

    Ok(())
}

fn truncate(s: &str, len: usize) -> String {
    if s.len() <= len {
        s.to_string()
    } else {
        format!("{}…", &s[..len])
    }
}
