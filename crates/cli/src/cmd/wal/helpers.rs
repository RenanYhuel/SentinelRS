use anyhow::Result;
use std::path::PathBuf;

use sentinel_agent::config::AgentConfig;

pub fn load_agent_config(config_path: Option<&str>) -> Result<AgentConfig> {
    let path = match config_path {
        Some(p) => std::path::PathBuf::from(p),
        None => default_agent_config_path(),
    };
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_yaml::from_str(&content)?)
}

pub fn default_agent_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sentinel")
        .join("agent.yml")
}

pub fn wal_dir(config_path: Option<&str>) -> Result<PathBuf> {
    let cfg = load_agent_config(config_path)?;
    Ok(PathBuf::from(&cfg.buffer.wal_dir))
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
