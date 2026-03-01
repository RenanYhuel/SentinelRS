use anyhow::{Context, Result};
use sentinel_agent::config::{load_from_file, AgentConfig};
use std::path::PathBuf;

pub fn default_config_path() -> PathBuf {
    if let Some(dir) = dirs::config_dir() {
        return dir.join("sentinel").join("agent.yml");
    }
    PathBuf::from("/etc/sentinel/agent.yml")
}

pub fn load_config(config_path: Option<&str>) -> Result<AgentConfig> {
    let path = config_path
        .map(PathBuf::from)
        .unwrap_or_else(default_config_path);

    load_from_file(&path).with_context(|| format!("loading config from {}", path.display()))
}

pub fn resolve_server(server_flag: Option<&str>, config_path: Option<&str>) -> Result<String> {
    if let Some(s) = server_flag {
        return Ok(s.to_string());
    }
    let cfg = load_config(config_path)?;
    Ok(cfg.server)
}

pub fn resolve_rest_url(server_flag: Option<&str>, config_path: Option<&str>) -> Result<String> {
    let base = resolve_server(server_flag, config_path)?;
    let url = base
        .replace("grpc://", "http://")
        .replace(":50051", ":8080");
    Ok(url)
}
