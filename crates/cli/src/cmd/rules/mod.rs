mod create;
mod delete;
mod get;
mod list;
mod update;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum RulesCmd {
    #[command(about = "List all alert rules")]
    List,
    #[command(about = "Get details for a specific rule")]
    Get(get::GetArgs),
    #[command(about = "Create a new alert rule")]
    Create(create::CreateArgs),
    #[command(about = "Update an existing alert rule")]
    Update(update::UpdateArgs),
    #[command(about = "Delete an alert rule")]
    Delete(delete::DeleteArgs),
}

pub async fn execute(cmd: RulesCmd, mode: OutputMode, server: Option<String>) -> Result<()> {
    match cmd {
        RulesCmd::List => list::run(mode, server).await,
        RulesCmd::Get(args) => get::run(args, mode, server).await,
        RulesCmd::Create(args) => create::run(args, mode, server).await,
        RulesCmd::Update(args) => update::run(args, mode, server).await,
        RulesCmd::Delete(args) => delete::run(args, mode, server).await,
    }
}
