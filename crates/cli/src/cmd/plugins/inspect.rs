use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args;

use crate::output::{print_json, OutputMode};

#[derive(Args)]
pub struct InspectArgs {
    #[arg(help = "Plugin name")]
    pub name: String,

    #[arg(long, default_value = "./plugins", help = "Plugins directory")]
    pub dir: String,
}

pub fn run(args: InspectArgs, mode: OutputMode) -> Result<()> {
    let dir = PathBuf::from(&args.dir);
    let manifest_path = dir.join(format!("{}.manifest.yml", args.name));

    if !manifest_path.exists() {
        bail!("Plugin '{}' not found in {}", args.name, args.dir);
    }

    let yaml = std::fs::read_to_string(&manifest_path)?;
    let manifest = sentinel_agent::plugin::PluginManifest::from_yaml(&yaml)
        .map_err(|e| anyhow::anyhow!("Invalid manifest: {e}"))?;

    let wasm_path = dir.join(format!("{}.wasm", args.name));
    let wasm_size = if wasm_path.exists() {
        std::fs::metadata(&wasm_path)?.len()
    } else {
        0
    };

    let sig_path = dir.join(format!("{}.sig", args.name));
    let signed = sig_path.exists();

    match mode {
        OutputMode::Json => {
            print_json(&serde_json::json!({
                "name": manifest.name,
                "version": manifest.version,
                "entry_fn": manifest.entry_fn,
                "capabilities": manifest.capabilities,
                "resource_limits": {
                    "max_memory_mb": manifest.resource_limits.max_memory_mb,
                    "timeout_ms": manifest.resource_limits.timeout_ms,
                    "max_metrics": manifest.resource_limits.max_metrics,
                },
                "metadata": manifest.metadata,
                "wasm_size_bytes": wasm_size,
                "signed": signed,
            }))?;
        }
        OutputMode::Human => {
            println!("Plugin: {}", manifest.name);
            println!("Version: {}", manifest.version);
            println!("Entry function: {}", manifest.entry_fn);
            println!("WASM size: {} bytes", wasm_size);
            println!("Signed: {}", if signed { "yes" } else { "no" });
            println!(
                "Memory limit: {} MB",
                manifest.resource_limits.max_memory_mb
            );
            println!("Timeout: {} ms", manifest.resource_limits.timeout_ms);
            println!("Max metrics: {}", manifest.resource_limits.max_metrics);

            if !manifest.capabilities.is_empty() {
                let caps: Vec<String> = manifest
                    .capabilities
                    .iter()
                    .map(|c| format!("{c:?}"))
                    .collect();
                println!("Capabilities: {}", caps.join(", "));
            }

            if !manifest.metadata.is_empty() {
                println!("Metadata:");
                for (k, v) in &manifest.metadata {
                    println!("  {k}: {v}");
                }
            }
        }
    }

    Ok(())
}
