pub mod actionable;
pub mod banner;
pub mod category;
pub mod colors;
pub mod config;
pub mod formatter;
pub mod icons;
pub mod latency;
pub mod timed;
pub mod visitor;

pub use banner::{print_banner, Component};
pub use config::{LogConfig, LogFormat};
pub use latency::track;
pub use timed::stopwatch;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

pub fn init(config: &LogConfig) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.default_level));

    match config.format {
        LogFormat::Clean => {
            tracing_subscriber::registry()
                .with(filter)
                .with(formatter::SpanFieldLayer)
                .with(tracing_subscriber::fmt::layer().event_format(formatter::SentinelFormatter))
                .init();
        }
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(filter)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
                .init();
        }
    }
}
