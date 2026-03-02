mod delete;
mod generate_install;
mod get;
mod list;
mod live;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum AgentsCmd {
    #[command(about = "List all registered agents")]
    List,
    #[command(about = "Get details for a specific agent")]
    Get(get::GetArgs),
    #[command(about = "Watch agent metrics in real-time")]
    Live(live::LiveArgs),
    #[command(about = "Remove an agent from the registry")]
    Delete(delete::DeleteArgs),
    #[command(about = "Generate a one-line install command")]
    GenerateInstall(generate_install::GenerateArgs),
}

pub async fn execute(cmd: AgentsCmd, mode: OutputMode, server: Option<String>) -> Result<()> {
    match cmd {
        AgentsCmd::List => list::run(mode, server).await,
        AgentsCmd::Get(args) => get::run(args, mode, server).await,
        AgentsCmd::Live(args) => live::run(args, mode, server).await,
        AgentsCmd::Delete(args) => delete::run(args, mode, server).await,
        AgentsCmd::GenerateInstall(args) => generate_install::run(args, mode, server).await,
    }
}
