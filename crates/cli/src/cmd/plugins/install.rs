use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Args;

use crate::output::{print_success, OutputMode};

#[derive(Args)]
pub struct InstallArgs {
    #[arg(help = "Path to .wasm file")]
    pub wasm_path: String,

    #[arg(long, help = "Path to manifest YAML file")]
    pub manifest: Option<String>,

    #[arg(long, default_value = "./plugins", help = "Plugins directory")]
    pub dir: String,

    #[arg(long, help = "Signing key for HMAC signature")]
    pub signing_key: Option<String>,
}

pub async fn run(args: InstallArgs, mode: OutputMode) -> Result<()> {
    let wasm_path = PathBuf::from(&args.wasm_path);
    if !wasm_path.exists() {
        bail!("WASM file not found: {}", args.wasm_path);
    }

    let wasm_bytes = std::fs::read(&wasm_path).context("Failed to read WASM file")?;

    let plugin_name = wasm_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("plugin")
        .to_string();

    let plugins_dir = PathBuf::from(&args.dir);
    std::fs::create_dir_all(&plugins_dir).context("Failed to create plugins directory")?;

    sentinel_agent::plugin::store_blob(&plugins_dir, &plugin_name, &wasm_bytes)?;

    if let Some(ref manifest_path) = args.manifest {
        let yaml = std::fs::read_to_string(manifest_path).context("Failed to read manifest")?;
        let _manifest = sentinel_agent::plugin::PluginManifest::from_yaml(&yaml)
            .map_err(|e| anyhow::anyhow!("Invalid manifest: {e}"))?;
        sentinel_agent::plugin::store_manifest(&plugins_dir, &plugin_name, &yaml)?;
    } else {
        let default_manifest =
            format!("name: {plugin_name}\nversion: \"1.0.0\"\nentry_fn: collect\n");
        sentinel_agent::plugin::store_manifest(&plugins_dir, &plugin_name, &default_manifest)?;
    }

    if let Some(ref key) = args.signing_key {
        let sig = sentinel_agent::plugin::sign_blob(&wasm_bytes, key.as_bytes());
        let sig_path = plugins_dir.join(format!("{plugin_name}.sig"));
        std::fs::write(sig_path, sig)?;
    }

    match mode {
        OutputMode::Json => {
            let json = serde_json::json!({
                "status": "installed",
                "name": plugin_name,
                "dir": args.dir,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputMode::Human => {
            print_success(&format!("Plugin '{plugin_name}' installed to {}", args.dir));
        }
    }

    Ok(())
}
