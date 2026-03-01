use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

use sentinel_agent::buffer::{compact, compute_stats, needs_compaction, Wal, WalMeta};
use sentinel_agent::batch::BatchComposer;
use crate::output::{OutputMode, print_json, print_success, build_table, spinner, theme, confirm};
use super::helpers;

#[derive(Subcommand)]
pub enum WalCmd {
    Stats(WalStatsArgs),
    Inspect(InspectArgs),
    Compact(CompactArgs),
    Meta(MetaArgs),
}

#[derive(clap::Args)]
pub struct WalStatsArgs;

#[derive(clap::Args)]
pub struct InspectArgs {
    #[arg(long, default_value = "20")]
    limit: usize,
}

#[derive(clap::Args)]
pub struct CompactArgs {
    #[arg(long)]
    force: bool,
    #[arg(long, help = "Skip confirmation prompt")]
    yes: bool,
}

#[derive(clap::Args)]
pub struct MetaArgs;

pub async fn execute(cmd: WalCmd, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    match cmd {
        WalCmd::Stats(args) => stats(args, mode, config_path),
        WalCmd::Inspect(args) => inspect(args, mode, config_path),
        WalCmd::Compact(args) => compact_cmd(args, mode, config_path),
        WalCmd::Meta(args) => meta(args, mode, config_path),
    }
}

fn wal_dir(config_path: Option<&str>) -> Result<PathBuf> {
    let cfg = helpers::load_config(config_path)?;
    Ok(PathBuf::from(&cfg.buffer.wal_dir))
}

fn stats(_args: WalStatsArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = wal_dir(config_path.as_deref())?;
    let wal = Wal::open(&dir, false, 16 * 1024 * 1024)?;
    let unacked = wal.unacked_count()? as u64;
    let s = compute_stats(&dir, unacked)?;

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "total_size_bytes": s.total_size_bytes,
            "segment_count": s.segment_count,
            "unacked_count": s.unacked_count,
        }))?,
        OutputMode::Human => {
            theme::print_header("WAL Statistics");
            theme::print_kv("Total size", &format_bytes(s.total_size_bytes));
            theme::print_kv("Segments", &s.segment_count.to_string());
            theme::print_kv_colored(
                "Unacked",
                &s.unacked_count.to_string(),
                s.unacked_count == 0,
            );
            println!();
        }
    }

    Ok(())
}

fn inspect(args: InspectArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = wal_dir(config_path.as_deref())?;
    let wal = Wal::open(&dir, false, 16 * 1024 * 1024)?;
    let entries = wal.iter_unacked()?;
    let limited: Vec<_> = entries.into_iter().take(args.limit).collect();

    match mode {
        OutputMode::Json => {
            let items: Vec<_> = limited
                .iter()
                .map(|(id, data)| {
                    let batch = BatchComposer::decode_batch(data).ok();
                    serde_json::json!({
                        "record_id": id,
                        "size_bytes": data.len(),
                        "batch_id": batch.as_ref().map(|b| b.batch_id.as_str()),
                        "agent_id": batch.as_ref().map(|b| b.agent_id.as_str()),
                        "metrics_count": batch.as_ref().map(|b| b.metrics.len()),
                    })
                })
                .collect();
            print_json(&items)?;
        }
        OutputMode::Human => {
            if limited.is_empty() {
                print_success("No unacked entries in WAL");
                return Ok(());
            }

            theme::print_header("WAL Entries");
            let mut table = build_table(&["Record ID", "Size", "Batch ID", "Metrics"]);
            for (id, data) in &limited {
                let batch = BatchComposer::decode_batch(data).ok();
                table.add_row(vec![
                    id.to_string(),
                    format_bytes(data.len() as u64),
                    batch.as_ref().map(|b| b.batch_id.clone()).unwrap_or_default(),
                    batch
                        .as_ref()
                        .map(|b| b.metrics.len().to_string())
                        .unwrap_or_default(),
                ]);
            }
            println!("{table}");
        }
    }

    Ok(())
}

fn compact_cmd(args: CompactArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = wal_dir(config_path.as_deref())?;

    if !args.force {
        let threshold = 64 * 1024 * 1024;
        let needed = needs_compaction(&dir, threshold)?;
        if !needed {
            match mode {
                OutputMode::Json => print_json(&serde_json::json!({"compacted": false, "reason": "threshold not reached"}))?,
                OutputMode::Human => print_success("Compaction not needed (threshold not reached). Use --force to override."),
            }
            return Ok(());
        }
    }

    if mode == OutputMode::Human && !args.yes {
        if !confirm::confirm_action("Compact WAL? This rewrites segment files.") {
            theme::print_dim("  Cancelled.");
            return Ok(());
        }
    }

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Compacting WAL...")),
        OutputMode::Json => None,
    };

    let meta = WalMeta::load(&dir)?;
    let new_meta = compact(&dir, &meta)?;
    new_meta.save(&dir)?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "WAL compacted successfully");
    }

    if mode == OutputMode::Json {
        print_json(&serde_json::json!({"compacted": true}))?;
    }

    Ok(())
}

fn meta(_args: MetaArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = wal_dir(config_path.as_deref())?;
    let m = WalMeta::load(&dir)?;

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "head_seq": m.head_seq,
            "tail_seq": m.tail_seq,
            "last_segment": m.last_segment,
            "acked_count": m.acked_ids.len(),
        }))?,
        OutputMode::Human => {
            theme::print_header("WAL Metadata");
            theme::print_kv("Head seq", &m.head_seq.to_string());
            theme::print_kv("Tail seq", &m.tail_seq.to_string());
            theme::print_kv("Last segment", &m.last_segment.to_string());
            theme::print_kv("Acked IDs", &m.acked_ids.len().to_string());
            println!();
        }
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
