use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

use sentinel_agent::batch::BatchComposer;
use sentinel_agent::buffer::Wal;
use sentinel_agent::exporter::GrpcClient;
use sentinel_common::proto::push_response::Status;
use crate::output::{OutputMode, print_json, spinner, progress, theme, confirm};
use super::helpers;

#[derive(Args)]
pub struct ForceSendArgs {
    #[arg(long, help = "Max batches to send (0 = all)")]
    limit: Option<usize>,
    #[arg(long, help = "Skip confirmation prompt")]
    yes: bool,
}

pub async fn execute(
    args: ForceSendArgs,
    mode: OutputMode,
    server: Option<String>,
    config_path: Option<String>,
) -> Result<()> {
    let cfg = helpers::load_config(config_path.as_deref())?;
    let endpoint = server.as_deref().unwrap_or(&cfg.server);
    let dir = PathBuf::from(&cfg.buffer.wal_dir);
    let mut wal = Wal::open(&dir, false, cfg.buffer.segment_size_mb * 1024 * 1024)?;
    let entries = wal.iter_unacked()?;

    if entries.is_empty() {
        match mode {
            OutputMode::Json => print_json(&serde_json::json!({"sent": 0}))?,
            OutputMode::Human => theme::print_dim("  No unacked entries to send"),
        }
        return Ok(());
    }

    let limit = args.limit.unwrap_or(0);
    let to_send: Vec<_> = if limit > 0 {
        entries.into_iter().take(limit).collect()
    } else {
        entries
    };

    if mode == OutputMode::Human && !args.yes {
        let msg = format!("Send {} batch(es) to server?", to_send.len());
        if !confirm::confirm_default_yes(&msg) {
            theme::print_dim("  Cancelled.");
            return Ok(());
        }
    }

    let creds = load_credentials()?;

    let secret = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &creds.secret,
    )
    .context("invalid base64 secret")?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Connecting to server...")),
        OutputMode::Json => None,
    };

    let mut client = GrpcClient::connect(
        endpoint,
        creds.agent_id.clone(),
        &secret,
        "default".to_string(),
    )
    .await
    .context("failed to connect")?;

    if let Some(sp) = &sp {
        spinner::finish_clear(sp);
    }

    let pb = match mode {
        OutputMode::Human => Some(progress::create(to_send.len() as u64, "Sending batches")),
        OutputMode::Json => None,
    };

    let mut sent = 0u64;
    let mut failed = 0u64;

    for (record_id, data) in &to_send {
        let batch = match BatchComposer::decode_batch(data) {
            Ok(b) => b,
            Err(_) => {
                failed += 1;
                if let Some(pb) = &pb {
                    pb.inc(1);
                }
                continue;
            }
        };

        match client.push_metrics(batch).await {
            Ok(resp) => match Status::try_from(resp.status) {
                Ok(Status::Ok) => {
                    wal.ack(*record_id);
                    sent += 1;
                }
                Ok(Status::Rejected) => {
                    wal.ack(*record_id);
                    failed += 1;
                }
                _ => {
                    failed += 1;
                }
            },
            Err(_) => {
                failed += 1;
            }
        }

        if let Some(pb) = &pb {
            pb.inc(1);
        }
    }

    wal.save_meta()?;

    if let Some(pb) = pb {
        progress::finish(&pb, "All batches processed");
    }

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "sent": sent,
            "failed": failed,
            "total": to_send.len(),
        }))?,
        OutputMode::Human => {
            println!();
            theme::print_kv("Sent", &sent.to_string());
            theme::print_kv("Failed", &failed.to_string());
            theme::print_kv("Total", &to_send.len().to_string());
        }
    }

    Ok(())
}

#[derive(serde::Deserialize)]
struct Credentials {
    agent_id: String,
    secret: String,
}

fn load_credentials() -> Result<Credentials> {
    let path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sentinel")
        .join("credentials.json");

    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read credentials from {}", path.display()))?;

    serde_json::from_str(&data).context("invalid credentials file")
}
