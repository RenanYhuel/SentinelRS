use anyhow::Result;
use clap::Subcommand;

use super::helpers;
use crate::output::{print_error, print_json, print_success, theme, OutputMode};

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
            theme::print_header("Agent Configuration");

            theme::print_section("Identity");
            theme::print_kv("Agent ID", cfg.agent_id.as_deref().unwrap_or("<not set>"));
            theme::print_kv("Server", &cfg.server);

            theme::print_section("Collection");
            theme::print_kv("Interval", &format!("{}s", cfg.collect.interval_seconds));
            theme::print_kv_colored(
                "CPU",
                &cfg.collect.metrics.cpu.to_string(),
                cfg.collect.metrics.cpu,
            );
            theme::print_kv_colored(
                "Memory",
                &cfg.collect.metrics.mem.to_string(),
                cfg.collect.metrics.mem,
            );
            theme::print_kv_colored(
                "Disk",
                &cfg.collect.metrics.disk.to_string(),
                cfg.collect.metrics.disk,
            );

            theme::print_section("Plugins");
            theme::print_kv("Directory", &cfg.plugins_dir);

            theme::print_section("Buffer (WAL)");
            theme::print_kv("Directory", &cfg.buffer.wal_dir);
            theme::print_kv(
                "Segment size",
                &format!("{} MB", cfg.buffer.segment_size_mb),
            );
            theme::print_kv(
                "Retention",
                &format!("{} days", cfg.buffer.max_retention_days),
            );

            theme::print_section("Security");
            theme::print_kv("Key store", &cfg.security.key_store);
            theme::print_kv(
                "Rotation check",
                &format!("every {} hours", cfg.security.rotation_check_interval_hours),
            );
            println!();
        }
    }

    Ok(())
}

fn validate(_args: ValidateArgs, mode: OutputMode, config_path: Option<String>) -> Result<()> {
    match helpers::load_config(config_path.as_deref()) {
        Ok(_cfg) => match mode {
            OutputMode::Json => {
                print_json(&serde_json::json!({"valid": true}))?;
            }
            OutputMode::Human => print_success("Configuration is valid"),
        },
        Err(e) => match mode {
            OutputMode::Json => {
                print_json(&serde_json::json!({"valid": false, "error": e.to_string()}))?;
            }
            OutputMode::Human => print_error(&format!("Invalid configuration: {e}")),
        },
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
