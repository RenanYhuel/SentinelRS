use anyhow::Result;
use clap::Args;

use crate::output::{print_json, spinner, OutputMode};
use sentinel_agent::config::{EncryptedFileStore, KeyStore};

#[derive(Args)]
pub struct RotateArgs {
    #[arg(long, help = "Key identifier")]
    pub key_id: String,

    #[arg(long, help = "Base64-encoded new secret")]
    pub secret: String,
}

pub fn run(args: RotateArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = key_store_dir(config_path.as_deref())?;
    let store = EncryptedFileStore::new(&dir, master_key());

    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &args.secret)
        .map_err(|e| anyhow::anyhow!("invalid base64: {e}"))?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Rotating key...")),
        OutputMode::Json => None,
    };

    store
        .store(&args.key_id, &decoded)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, &format!("Key '{}' rotated", args.key_id));
    }

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "rotated": true,
            "key_id": args.key_id,
        }))?,
        OutputMode::Human => {}
    }

    Ok(())
}

pub fn key_store_dir(config_path: Option<&str>) -> Result<std::path::PathBuf> {
    if let Some(p) = config_path {
        let cfg_dir = std::path::Path::new(p)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        return Ok(cfg_dir.join("keys"));
    }
    let dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("sentinel")
        .join("keys");
    Ok(dir)
}

pub fn master_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    let seed = b"sentinel-cli-default-master-key!";
    key.copy_from_slice(seed);
    key
}
