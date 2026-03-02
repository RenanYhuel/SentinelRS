mod test;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum NotifiersCmd {
    #[command(about = "Send a test notification")]
    Test(test::TestArgs),
}

pub async fn execute(cmd: NotifiersCmd, mode: OutputMode, server: Option<String>) -> Result<()> {
    match cmd {
        NotifiersCmd::Test(args) => test::run(args, mode, server).await,
    }
}
