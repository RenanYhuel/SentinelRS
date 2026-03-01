use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use sentinel_agent::buffer::{compute_stats, Wal};
use crate::output::{OutputMode, print_json, print_success, print_info};
use super::helpers;

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

    let server_up = match &rest_base {
        Some(url) => reqwest::get(&format!("{url}/healthz")).await.is_ok(),
        None => false,
    };

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
            print_success("SentinelRS Status");
            print_info("Agent ID", &agent_id);
            if server_up {
                print_info("Server", "reachable");
            } else {
                print_info("Server", "unreachable");
            }
            if let Some(w) = &wal_info {
                print_info("WAL segments", &w.segment_count.to_string());
                print_info("WAL unacked", &w.unacked_count.to_string());
            } else {
                print_info("WAL", "unavailable");
            }
        }
    }

    Ok(())
}
