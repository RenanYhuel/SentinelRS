mod inspect;
mod install;
mod list;
mod remove;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum PluginsCmd {
    #[command(about = "Install a plugin from a local path")]
    Install(install::InstallArgs),

    #[command(about = "List installed plugins")]
    List(list::ListArgs),

    #[command(about = "Show plugin details")]
    Inspect(inspect::InspectArgs),

    #[command(about = "Remove an installed plugin")]
    Remove(remove::RemoveArgs),
}

pub async fn execute(cmd: PluginsCmd, mode: OutputMode) -> Result<()> {
    match cmd {
        PluginsCmd::Install(args) => install::run(args, mode).await,
        PluginsCmd::List(args) => list::run(args, mode),
        PluginsCmd::Inspect(args) => inspect::run(args, mode),
        PluginsCmd::Remove(args) => remove::run(args, mode),
    }
}
