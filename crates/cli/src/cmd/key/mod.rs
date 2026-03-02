mod delete;
mod list;
mod rotate;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum KeyCmd {
    #[command(about = "Rotate key for an agent")]
    Rotate(rotate::RotateArgs),
    #[command(about = "List stored encryption keys")]
    List,
    #[command(about = "Delete a stored key")]
    Delete(delete::DeleteArgs),
}

pub fn execute(cmd: KeyCmd, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    match cmd {
        KeyCmd::Rotate(args) => rotate::run(args, mode, config_path),
        KeyCmd::List => list::run(mode, config_path),
        KeyCmd::Delete(args) => delete::run(args, mode, config_path),
    }
}
