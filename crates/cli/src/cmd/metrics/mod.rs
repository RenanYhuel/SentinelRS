mod live;
mod show;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum MetricsCmd {
    #[command(about = "Display server metrics snapshot")]
    Show,
    #[command(about = "Watch metrics in real-time")]
    Live(live::LiveArgs),
}

pub async fn execute(cmd: MetricsCmd, mode: OutputMode, server: Option<String>) -> Result<()> {
    match cmd {
        MetricsCmd::Show => show::run(mode, server).await,
        MetricsCmd::Live(args) => live::run(args, mode, server).await,
    }
}
