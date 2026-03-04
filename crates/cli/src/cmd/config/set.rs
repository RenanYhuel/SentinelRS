use anyhow::Result;
use clap::Args;

use crate::output::{print_success, OutputMode};
use crate::store;

#[derive(Args)]
pub struct SetArgs {
    #[arg(help = "Dotted key (e.g. server.url)")]
    pub key: String,

    #[arg(help = "Value to set")]
    pub value: String,
}

pub fn run(args: SetArgs, mode: OutputMode) -> Result<()> {
    let mut cfg = store::load().unwrap_or_default();

    cfg.set_dotted(&args.key, &args.value)
        .map_err(|e| anyhow::anyhow!(e))?;

    store::save(&cfg)?;

    if mode == OutputMode::Human {
        print_success(&format!("{} = {}", args.key, args.value));
    }

    Ok(())
}
