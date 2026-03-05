mod edit;
mod path;
mod reset;
mod set;
mod show;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum ConfigCmd {
    #[command(about = "Show current CLI configuration")]
    Show(show::ShowArgs),
    #[command(about = "Set a configuration value")]
    Set(set::SetArgs),
    #[command(about = "Edit CLI configuration interactively")]
    Edit,
    #[command(about = "Print config file path")]
    Path,
    #[command(about = "Reset CLI configuration to defaults")]
    Reset,
}

pub async fn execute(cmd: ConfigCmd, mode: OutputMode) -> Result<()> {
    match cmd {
        ConfigCmd::Show(args) => show::run(args, mode),
        ConfigCmd::Set(args) => set::run(args, mode),
        ConfigCmd::Edit => edit::run(mode).await,
        ConfigCmd::Path => path::run(mode),
        ConfigCmd::Reset => reset::run(mode),
    }
}
