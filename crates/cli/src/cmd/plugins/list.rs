use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::output::{build_table, print_json, OutputMode};

#[derive(Args)]
pub struct ListArgs {
    #[arg(long, default_value = "./plugins", help = "Plugins directory")]
    pub dir: String,
}

pub fn run(args: ListArgs, mode: OutputMode) -> Result<()> {
    let dir = PathBuf::from(&args.dir);
    let plugins = sentinel_agent::plugin::discovery::list_installed(&dir);

    match mode {
        OutputMode::Json => {
            let items: Vec<serde_json::Value> = plugins
                .iter()
                .map(|(name, m)| {
                    serde_json::json!({
                        "name": name,
                        "version": m.version,
                        "entry_fn": m.entry_fn,
                        "capabilities": m.capabilities,
                    })
                })
                .collect();
            print_json(&serde_json::json!({ "plugins": items }))?;
        }
        OutputMode::Human => {
            if plugins.is_empty() {
                println!("No plugins installed in {}", args.dir);
                return Ok(());
            }
            let headers = &["Name", "Version", "Entry", "Capabilities"];
            let mut table = build_table(headers);
            for (name, m) in &plugins {
                let caps: Vec<String> = m.capabilities.iter().map(|c| format!("{c:?}")).collect();
                table.add_row(vec![
                    name.clone(),
                    m.version.clone(),
                    m.entry_fn.clone(),
                    if caps.is_empty() {
                        "-".to_string()
                    } else {
                        caps.join(", ")
                    },
                ]);
            }
            println!("{table}");
        }
    }

    Ok(())
}
