use anyhow::{Context, Result};
use std::path::PathBuf;

use super::config::CliConfig;

pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".sentinel")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

fn legacy_yaml_path() -> PathBuf {
    config_dir().join("cli.yml")
}

pub fn load() -> Result<CliConfig> {
    let toml_path = config_path();
    if toml_path.exists() {
        let content = std::fs::read_to_string(&toml_path).context("cannot read config.toml")?;
        return toml::from_str(&content).context("invalid config.toml format");
    }

    let yaml_path = legacy_yaml_path();
    if yaml_path.exists() {
        let cfg = migrate_from_yaml(&yaml_path)?;
        save(&cfg)?;
        return Ok(cfg);
    }

    anyhow::bail!("CLI not configured. Run `sentinel init` first.")
}

fn migrate_from_yaml(path: &PathBuf) -> Result<CliConfig> {
    let content = std::fs::read_to_string(path).context("cannot read legacy cli.yml")?;
    let legacy: LegacyConfig =
        serde_yaml::from_str(&content).context("invalid legacy cli.yml format")?;

    let mut cfg = CliConfig::default();
    cfg.server.url = legacy.server_url;
    cfg.defaults.output_format = legacy.output;
    Ok(cfg)
}

#[derive(serde::Deserialize)]
struct LegacyConfig {
    #[serde(default = "default_url")]
    server_url: String,
    #[serde(default = "default_out")]
    output: String,
}

fn default_url() -> String {
    "http://localhost:8080".into()
}

fn default_out() -> String {
    "human".into()
}

pub fn save(cfg: &CliConfig) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let toml_str = toml::to_string_pretty(cfg).context("cannot serialize config")?;
    std::fs::write(config_path(), toml_str)?;
    Ok(())
}

pub fn exists() -> bool {
    config_path().exists() || legacy_yaml_path().exists()
}
