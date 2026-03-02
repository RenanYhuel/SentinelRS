mod agents;
mod status;
mod watch;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum ClusterCmd {
    #[command(about = "Show cluster overview and stats")]
    Status,
    #[command(about = "List connected agents in the cluster")]
    Agents,
    #[command(about = "Watch real-time cluster events (SSE stream)")]
    Watch,
}

pub async fn execute(cmd: ClusterCmd, mode: OutputMode, server: Option<String>) -> Result<()> {
    match cmd {
        ClusterCmd::Status => status::run(mode, server).await,
        ClusterCmd::Agents => agents::run(mode, server).await,
        ClusterCmd::Watch => watch::run(mode, server).await,
    }
}
