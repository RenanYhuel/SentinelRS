use anyhow::Result;

use crate::output::{print_json, theme, OutputMode};
use crate::store;

pub fn run(mode: OutputMode) -> Result<()> {
    let cfg = store::load()?;

    match mode {
        OutputMode::Json => print_json(&cfg)?,
        OutputMode::Human => {
            theme::print_header("CLI Configuration");
            theme::print_kv("Server URL", &cfg.server_url);
            theme::print_kv("Output", &cfg.output);
            println!();
            theme::print_dim(&format!("  File: {}", store::config_path().display()));
            println!();
        }
    }

    Ok(())
}
