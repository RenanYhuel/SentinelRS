mod api;
mod batch;
mod buffer;
mod collector;
mod config;
mod exporter;
mod plugin;
mod scheduler;
mod security;

use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .json()
        .init();

    tracing::info!("SentinelRS agent starting");
}
