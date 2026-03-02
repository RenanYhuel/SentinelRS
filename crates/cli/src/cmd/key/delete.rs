use anyhow::Result;
use clap::Args;

use crate::output::{confirm, print_json, print_success, theme, OutputMode};
use sentinel_agent::config::{EncryptedFileStore, KeyStore};

#[derive(Args)]
pub struct DeleteArgs {
    #[arg(help = "Key ID to delete")]
    pub key_id: String,

    #[arg(long, help = "Skip confirmation")]
    pub yes: bool,
}

pub fn run(args: DeleteArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    if mode == OutputMode::Human && !args.yes {
        let msg = format!("Delete key '{}'? This cannot be undone.", args.key_id);
        if !confirm::confirm_action(&msg) {
            theme::print_dim("  Cancelled.");
            return Ok(());
        }
    }

    let dir = super::rotate::key_store_dir(config_path.as_deref())?;
    let store = EncryptedFileStore::new(&dir, super::rotate::master_key());

    store
        .delete(&args.key_id)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "deleted": true,
            "key_id": args.key_id,
        }))?,
        OutputMode::Human => print_success(&format!("Key '{}' deleted", args.key_id)),
    }

    Ok(())
}
