mod compact;
pub(crate) mod helpers;
mod inspect;
mod meta;
mod stats;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum WalCmd {
    #[command(about = "Show WAL statistics")]
    Stats,
    #[command(about = "Inspect unacked WAL entries")]
    Inspect(inspect::InspectArgs),
    #[command(about = "Compact WAL segments")]
    Compact(compact::CompactArgs),
    #[command(about = "Show WAL metadata")]
    Meta,
}

pub fn execute(cmd: WalCmd, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    match cmd {
        WalCmd::Stats => stats::run(mode, config_path),
        WalCmd::Inspect(args) => inspect::run(args, mode, config_path),
        WalCmd::Compact(args) => compact::run(args, mode, config_path),
        WalCmd::Meta => meta::run(mode, config_path),
    }
}
