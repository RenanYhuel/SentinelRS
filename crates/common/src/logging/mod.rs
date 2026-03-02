pub mod banner;
pub mod category;
pub mod colors;
pub mod config;
pub mod formatter;
pub mod icons;
pub mod timed;
pub mod visitor;

pub use banner::{print_banner, Component};
pub use config::{LogConfig, LogFormat};
pub use timed::stopwatch;

use tracing_subscriber::EnvFilter;

pub fn init(config: &LogConfig) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.default_level));

    match config.format {
        LogFormat::Clean => {
            tracing_subscriber::fmt()
                .event_format(formatter::SentinelFormatter)
                .with_env_filter(filter)
                .init();
        }
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(filter)
                .init();
        }
    }
}
