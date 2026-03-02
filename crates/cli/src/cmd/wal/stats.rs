use anyhow::Result;

use crate::output::{print_json, theme, OutputMode};
use sentinel_agent::buffer::{compute_stats, Wal};

use super::helpers::{format_bytes, wal_dir};

pub fn run(mode: OutputMode, config_path: Option<String>) -> Result<()> {
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
