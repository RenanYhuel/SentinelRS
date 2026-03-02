use std::path::Path;

use sentinel_agent::persistence::VolumeLayout;
use sentinel_common::logging::{self, Component, LogConfig};

#[tokio::main]
async fn main() {
    let args = sentinel_agent::cli::parse();

    let log_config = LogConfig::from_env();
    logging::print_banner(Component::Agent, env!("CARGO_PKG_VERSION"));
    logging::init(&log_config);

    tracing::info!(target: "system", "Starting SentinelRS Agent");

    let config_dir = args
        .config_path
        .parent()
        .unwrap_or_else(|| Path::new("/etc/sentinel"));

    let layout = VolumeLayout::new(config_dir);

    // Step 1 — Verify / initialize volume
    if let Err(e) = sentinel_agent::persistence::volume::initialize(&layout, None) {
        tracing::warn!(target: "boot", error = %e, "Volume initialization warning");
    }
    tracing::info!(target: "boot", volume = %config_dir.display(), "Volume ready");

    // Step 2 — Config: load or bootstrap
    let config_path = match sentinel_agent::bootstrap::run_if_needed(&args.config_path).await {
        Ok(Some(new_path)) => {
            tracing::info!(target: "boot", "Bootstrap complete, loading provisioned config");
            new_path
        }
        Ok(None) => args.config_path.clone(),
        Err(e) => {
            tracing::error!(target: "boot", error = %e, "Bootstrap failed");
            std::process::exit(1);
        }
    };

    let config = match sentinel_agent::config::load_from_file(&config_path) {
        Ok(cfg) => {
            tracing::info!(target: "boot", "Configuration loaded from {}", config_path.display());
            cfg
        }
        Err(e) => {
            tracing::error!(target: "boot", error = %e, "Failed to load configuration");
            std::process::exit(1);
        }
    };

    // Steps 3-7 — WAL, gRPC, collector, heartbeat, send loop (in run::run)
    if let Err(e) = sentinel_agent::run::run(config, args.legacy_mode).await {
        tracing::error!(target: "system", error = %e, "Agent error");
        std::process::exit(1);
    }
}
