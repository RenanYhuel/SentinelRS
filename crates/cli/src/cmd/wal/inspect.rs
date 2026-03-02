use anyhow::Result;
use clap::Args;

use crate::output::{build_table, print_json, print_success, theme, OutputMode};
use sentinel_agent::batch::BatchComposer;
use sentinel_agent::buffer::Wal;

use super::helpers::{format_bytes, wal_dir};

#[derive(Args)]
pub struct InspectArgs {
    #[arg(long, default_value = "20")]
    pub limit: usize,
}

pub fn run(args: InspectArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
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
                    batch
                        .as_ref()
                        .map(|b| b.batch_id.clone())
                        .unwrap_or_default(),
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
