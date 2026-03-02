use anyhow::Result;
use clap::Args;

use crate::output::{print_error, print_success, theme, OutputMode};
use sentinel_agent::buffer::{compact, needs_compaction, WalMeta};

use super::helpers::wal_dir;

#[derive(Args)]
pub struct CompactArgs {
    #[arg(long)]
    pub force: bool,

    #[arg(long, short)]
    pub yes: bool,
}

pub fn run(args: CompactArgs, _mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = wal_dir(config_path.as_deref())?;

    if !args.force {
        let should = needs_compaction(&dir, 64 * 1024 * 1024)?;
        if !should {
            print_success("WAL does not need compaction");
            return Ok(());
        }
    }

    if !args.yes {
        let proceed = crate::output::confirm::confirm_action("Compact WAL? This cannot be undone");
        if !proceed {
            return Ok(());
        }
    }

    theme::print_header("WAL Compaction");
    let meta = WalMeta::load(&dir)?;
    match compact(&dir, &meta) {
        Ok(new_meta) => {
            print_success(&format!(
                "Compacted WAL — new head_seq={}, unacked={}",
                new_meta.head_seq,
                new_meta.unacked_count()
            ));
        }
        Err(e) => print_error(&format!("Compaction failed: {e}")),
    }

    Ok(())
}
