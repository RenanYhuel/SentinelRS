use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

use sentinel_agent::config::{EncryptedFileStore, KeyStore};
use crate::output::{OutputMode, print_json, print_success, print_info};
use super::helpers;

#[derive(Subcommand)]
pub enum KeyCmd {
    Rotate(RotateArgs),
    List(ListArgs),
    Delete(DeleteKeyArgs),
}

#[derive(clap::Args)]
pub struct RotateArgs {
    #[arg(long, help = "New key ID")]
    key_id: String,

    #[arg(long, help = "New secret (base64-encoded)")]
    secret: String,
}

#[derive(clap::Args)]
pub struct ListArgs;

#[derive(clap::Args)]
pub struct DeleteKeyArgs {
    #[arg(help = "Key ID to delete")]
    key_id: String,
}

pub async fn execute(
    cmd: KeyCmd,
    mode: OutputMode,
    config_path: Option<String>,
) -> Result<()> {
    match cmd {
        KeyCmd::Rotate(args) => rotate(args, mode, config_path),
        KeyCmd::List(args) => list(args, mode, config_path),
        KeyCmd::Delete(args) => delete(args, mode, config_path),
    }
}

fn key_store_dir(config_path: Option<&str>) -> Result<PathBuf> {
    let cfg = helpers::load_config(config_path)?;
    let dir = if cfg.security.key_store == "auto" {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel")
            .join("keys")
    } else {
        PathBuf::from(&cfg.security.key_store)
    };
    Ok(dir)
}

fn master_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    if let Ok(env_key) = std::env::var("SENTINEL_MASTER_KEY") {
        let bytes = env_key.as_bytes();
        let len = bytes.len().min(32);
        key[..len].copy_from_slice(&bytes[..len]);
    }
    key
}

fn rotate(args: RotateArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = key_store_dir(config_path.as_deref())?;
    let store = EncryptedFileStore::new(&dir, master_key());

    let secret_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &args.secret,
    )
    .context("invalid base64 secret")?;

    store
        .store(&args.key_id, &secret_bytes)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "rotated": true,
            "key_id": args.key_id,
        }))?,
        OutputMode::Human => {
            print_success("Key rotated successfully");
            print_info("Key ID", &args.key_id);
            print_info("Store", &dir.display().to_string());
        }
    }

    Ok(())
}

fn list(_args: ListArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = key_store_dir(config_path.as_deref())?;

    let mut keys = Vec::new();
    if dir.exists() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".enc") {
                keys.push(name.trim_end_matches(".enc").to_string());
            }
        }
    }

    match mode {
        OutputMode::Json => print_json(&keys)?,
        OutputMode::Human => {
            if keys.is_empty() {
                print_success("No keys in store");
            } else {
                print_success(&format!("{} key(s) found:", keys.len()));
                for k in &keys {
                    print_info("Key ID", k);
                }
            }
        }
    }

    Ok(())
}

fn delete(args: DeleteKeyArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = key_store_dir(config_path.as_deref())?;
    let store = EncryptedFileStore::new(&dir, master_key());

    store
        .delete(&args.key_id)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({"deleted": true, "key_id": args.key_id}))?,
        OutputMode::Human => print_success(&format!("Key '{}' deleted", args.key_id)),
    }

    Ok(())
}
