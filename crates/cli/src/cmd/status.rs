use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use super::helpers;
use crate::output::{print_json, spinner, theme, OutputMode};
use sentinel_agent::buffer::{compute_stats, Wal};

#[derive(Args)]
pub struct StatusArgs;

pub async fn execute(
    _args: StatusArgs,
    mode: OutputMode,
    server: Option<String>,
    config_path: Option<String>,
) -> Result<()> {
    let cfg = helpers::load_config(config_path.as_deref()).ok();
    let rest_base = helpers::resolve_rest_url(server.as_deref(), config_path.as_deref()).ok();

    if mode == OutputMode::Human {
        theme::print_header("SentinelRS Status");
    }

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Checking server connectivity...")),
        OutputMode::Json => None,
    };

    let server_up = match &rest_base {
        Some(url) => reqwest::get(&format!("{url}/healthz")).await.is_ok(),
        None => false,
    };

    if let Some(sp) = sp {
        if server_up {
            spinner::finish_ok(&sp, "Server reachable");
        } else {
            spinner::finish_err(&sp, "Server unreachable");
        }
    }

    let wal_info = cfg.as_ref().and_then(|c| {
        let dir = PathBuf::from(&c.buffer.wal_dir);
        let wal = Wal::open(&dir, false, c.buffer.segment_size_mb * 1024 * 1024).ok()?;
        let unacked = wal.unacked_count().ok()? as u64;
        compute_stats(&dir, unacked).ok()
    });

    let agent_id = cfg
        .as_ref()
        .and_then(|c| c.agent_id.clone())
        .unwrap_or_else(|| "<not configured>".into());

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "agent_id": agent_id,
            "server_reachable": server_up,
            "wal": wal_info.as_ref().map(|w| serde_json::json!({
                "total_size_bytes": w.total_size_bytes,
                "segment_count": w.segment_count,
                "unacked_count": w.unacked_count,
            })),
        }))?,
        OutputMode::Human => {
            println!();
            theme::print_section("Agent");
            theme::print_kv("Agent ID", &agent_id);

            theme::print_section("Server");
            theme::print_kv_colored(
                "Status",
                if server_up { "online" } else { "offline" },
                server_up,
            );

            theme::print_section("WAL");
            if let Some(w) = &wal_info {
                theme::print_kv("Segments", &w.segment_count.to_string());
                theme::print_kv_colored(
                    "Unacked",
                    &w.unacked_count.to_string(),
                    w.unacked_count == 0,
                );
            } else {
                theme::print_kv_colored("Status", "unavailable", false);
            }
            println!();
        }
    }

    Ok(())
}
