use sentinel_common::logging::{self, Component, LogConfig};

#[tokio::main]
async fn main() {
    let args = sentinel_agent::cli::parse();

    let log_config = LogConfig::from_env();
    logging::print_banner(Component::Agent, env!("CARGO_PKG_VERSION"));
    logging::init(&log_config);

    tracing::info!(target: "system", "Starting SentinelRS Agent");

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
            tracing::info!(target: "cfg", "Configuration loaded from {}", config_path.display());
            cfg
        }
        Err(e) => {
            tracing::error!(target: "cfg", error = %e, "Failed to load configuration");
            std::process::exit(1);
        }
    };

    if let Err(e) = sentinel_agent::run::run(config, args.legacy_mode).await {
        tracing::error!(target: "system", error = %e, "Agent error");
        std::process::exit(1);
    }
}
