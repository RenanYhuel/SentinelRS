use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::output::{print_error, print_success, OutputMode};

#[derive(Args)]
pub struct RemoveArgs {
    #[arg(help = "Plugin name")]
    pub name: String,

    #[arg(long, default_value = "./plugins", help = "Plugins directory")]
    pub dir: String,
}

pub fn run(args: RemoveArgs, mode: OutputMode) -> Result<()> {
    let dir = PathBuf::from(&args.dir);

    match sentinel_agent::plugin::discovery::remove_plugin(&dir, &args.name) {
        Ok(()) => match mode {
            OutputMode::Json => {
                let json = serde_json::json!({
                    "status": "removed",
                    "name": args.name,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
            OutputMode::Human => {
                print_success(&format!("Plugin '{}' removed", args.name));
            }
        },
        Err(e) => {
            print_error(&format!("Failed to remove plugin '{}': {e}", args.name));
        }
    }

    Ok(())
}
