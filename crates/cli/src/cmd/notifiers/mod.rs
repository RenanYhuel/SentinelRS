mod create;
mod delete;
mod link;
mod list;
mod test;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum NotifiersCmd {
    #[command(about = "Create a notifier channel", visible_alias = "add")]
    Create(create::CreateArgs),

    #[command(about = "List notifier channels", visible_alias = "ls")]
    List,

    #[command(about = "Delete a notifier channel", visible_alias = "rm")]
    Delete(delete::DeleteArgs),

    #[command(about = "Link notifiers to a rule")]
    Link(link::LinkArgs),

    #[command(about = "Send a test notification")]
    Test(test::TestArgs),
}

pub async fn execute(cmd: NotifiersCmd, mode: OutputMode, server: Option<String>) -> Result<()> {
    match cmd {
        NotifiersCmd::Create(args) => create::run(args, mode, server).await,
        NotifiersCmd::List => list::run(mode, server).await,
        NotifiersCmd::Delete(args) => delete::run(args, mode, server).await,
        NotifiersCmd::Link(args) => link::run(args, mode, server).await,
        NotifiersCmd::Test(args) => test::run(args, mode, server).await,
    }
}
