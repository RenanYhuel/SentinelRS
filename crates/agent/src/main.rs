use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let args = sentinel_agent::cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

    tracing::info!("SentinelRS agent starting");

    let config = match sentinel_agent::config::load_from_file(&args.config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!(error = %e, "failed to load configuration");
            std::process::exit(1);
        }
    };

    if let Err(e) = sentinel_agent::run::run(config).await {
        tracing::error!(error = %e, "agent error");
        std::process::exit(1);
    }
}
