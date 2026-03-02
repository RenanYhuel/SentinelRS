#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogFormat {
    Clean,
    Json,
}

pub struct LogConfig {
    pub format: LogFormat,
    pub default_level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Clean,
            default_level: "info".to_owned(),
        }
    }
}

impl LogConfig {
    pub fn from_env() -> Self {
        let format = match std::env::var("SENTINEL_LOG_FORMAT").as_deref() {
            Ok("json") => LogFormat::Json,
            _ => LogFormat::Clean,
        };
        let default_level = std::env::var("SENTINEL_LOG_LEVEL").unwrap_or_else(|_| "info".into());
        Self {
            format,
            default_level,
        }
    }
}
