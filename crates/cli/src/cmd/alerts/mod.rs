mod get;
mod list;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum AlertsCmd {
    #[command(about = "List recent alerts")]
    List(list::ListArgs),
    #[command(about = "Get details for a specific alert")]
    Get(get::GetArgs),
}

pub async fn execute(cmd: AlertsCmd, mode: OutputMode, server: Option<String>) -> Result<()> {
    match cmd {
        AlertsCmd::List(args) => list::run(args, mode, server).await,
        AlertsCmd::Get(args) => get::run(args, mode, server).await,
    }
}
