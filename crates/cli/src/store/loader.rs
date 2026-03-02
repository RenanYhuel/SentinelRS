use anyhow::{Context, Result};
use std::path::PathBuf;

use super::config::CliConfig;

pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".sentinel")
}

pub fn config_path() -> PathBuf {
    config_dir().join("cli.yml")
}

pub fn load() -> Result<CliConfig> {
    let path = config_path();
    let content =
        std::fs::read_to_string(&path).context("CLI not configured. Run `sentinel init` first.")?;
    serde_yaml::from_str(&content).context("invalid cli.yml format")
}

pub fn save(cfg: &CliConfig) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let yaml = serde_yaml::to_string(cfg)?;
    std::fs::write(config_path(), yaml)?;
    Ok(())
}

pub fn exists() -> bool {
    config_path().exists()
}
