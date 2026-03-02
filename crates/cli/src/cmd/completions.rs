use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::Opts;

pub fn execute(shell: Shell) -> Result<()> {
    let mut cmd = Opts::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, &name, &mut std::io::stdout());
    Ok(())
}
