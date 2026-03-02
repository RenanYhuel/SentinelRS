use anyhow::Result;

use crate::output::{confirm, print_success, theme, OutputMode};
use crate::store::{self, CliConfig};

pub fn run(mode: OutputMode) -> Result<()> {
    if mode == OutputMode::Human && !confirm::confirm_action("Reset CLI configuration to defaults?")
    {
        theme::print_dim("  Cancelled.");
        return Ok(());
    }

    let defaults = CliConfig::default();
    store::save(&defaults)?;
    print_success("Configuration reset to defaults");

    Ok(())
}
