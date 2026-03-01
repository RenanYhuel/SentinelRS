use anyhow::Result;
use clap::Subcommand;

use crate::output::{OutputMode, print_json, print_success, print_error, print_info};
use super::helpers;

#[derive(Subcommand)]
pub enum ConfigCmd {
    Show(ShowArgs),
    Validate(ValidateArgs),
    Path,
}

#[derive(clap::Args)]
pub struct ShowArgs;

#[derive(clap::Args)]
pub struct ValidateArgs;

pub async fn execute(cmd: ConfigCmd, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    match cmd {
        ConfigCmd::Show(args) => show(args, mode, config_path),
        ConfigCmd::Validate(args) => validate(args, mode, config_path),
        ConfigCmd::Path => path(mode, config_path),
    }
}

fn show(_args: ShowArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let cfg = helpers::load_config(config_path.as_deref())?;

    match mode {
        OutputMode::Json => print_json(&serde_json::to_value(&cfg)?)?,
        OutputMode::Human => {
            print_success("Agent configuration:");
            print_info("Agent ID", &cfg.agent_id.as_deref().unwrap_or("<not set>"));
            print_info("Server", &cfg.server);
            print_info("Collect interval", &format!("{}s", cfg.collect.interval_seconds));
            print_info("CPU", &cfg.collect.metrics.cpu.to_string());
            print_info("Memory", &cfg.collect.metrics.mem.to_string());
            print_info("Disk", &cfg.collect.metrics.disk.to_string());
            print_info("Plugins dir", &cfg.plugins_dir);
            print_info("WAL dir", &cfg.buffer.wal_dir);
            print_info("Segment size", &format!("{} MB", cfg.buffer.segment_size_mb));
            print_info("Retention", &format!("{} days", cfg.buffer.max_retention_days));
            print_info("Key store", &cfg.security.key_store);
            print_info(
                "Rotation check",
                &format!("every {} hours", cfg.security.rotation_check_interval_hours),
            );
        }
    }

    Ok(())
}

fn validate(_args: ValidateArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    match helpers::load_config(config_path.as_deref()) {
        Ok(_cfg) => {
            match mode {
                OutputMode::Json => {
                    print_json(&serde_json::json!({"valid": true}))?;
                }
                OutputMode::Human => print_success("Configuration is valid"),
            }
        }
        Err(e) => {
            match mode {
                OutputMode::Json => {
                    print_json(&serde_json::json!({"valid": false, "error": e.to_string()}))?;
                }
                OutputMode::Human => print_error(&format!("Invalid configuration: {e}")),
            }
        }
    }

    Ok(())
}

fn path(_mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let p = config_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(helpers::default_config_path);

    println!("{}", p.display());
    Ok(())
}
