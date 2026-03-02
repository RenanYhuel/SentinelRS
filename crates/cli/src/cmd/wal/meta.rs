use anyhow::Result;

use crate::output::{print_json, theme, OutputMode};
use sentinel_agent::buffer::WalMeta;

use super::helpers::wal_dir;

pub fn run(mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = wal_dir(config_path.as_deref())?;
    let meta = WalMeta::load(&dir)?;

    match mode {
        OutputMode::Json => {
            print_json(&serde_json::json!({
                "head_seq": meta.head_seq,
                "tail_seq": meta.tail_seq,
                "last_segment": meta.last_segment,
                "acked_count": meta.acked_ids.len(),
                "unacked_count": meta.unacked_count(),
            }))?;
        }
        OutputMode::Human => {
            theme::print_header("WAL Metadata");
            theme::print_kv("Head Seq", &meta.head_seq.to_string());
            theme::print_kv("Tail Seq", &meta.tail_seq.to_string());
            theme::print_kv("Last Segment", &meta.last_segment.to_string());
            theme::print_kv("Acked IDs", &meta.acked_ids.len().to_string());
            theme::print_kv("Unacked", &meta.unacked_count().to_string());
        }
    }

    Ok(())
}
