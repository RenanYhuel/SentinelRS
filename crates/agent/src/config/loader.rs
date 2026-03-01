use std::path::Path;
use super::schema::AgentConfig;

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Parse(serde_yaml::Error),
    Validation(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Parse(e) => write!(f, "parse: {e}"),
            Self::Validation(msg) => write!(f, "validation: {msg}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_yaml::Error> for LoadError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Parse(e)
    }
}

pub fn load_from_file(path: &Path) -> Result<AgentConfig, LoadError> {
    let contents = std::fs::read_to_string(path)?;
    load_from_str(&contents)
}

pub fn load_from_str(yaml: &str) -> Result<AgentConfig, LoadError> {
    let cfg: AgentConfig = serde_yaml::from_str(yaml)?;
    validate(&cfg)?;
    Ok(cfg)
}

fn validate(cfg: &AgentConfig) -> Result<(), LoadError> {
    if cfg.server.is_empty() {
        return Err(LoadError::Validation("server URL must not be empty".into()));
    }
    if cfg.collect.interval_seconds == 0 {
        return Err(LoadError::Validation(
            "collect.interval_seconds must be > 0".into(),
        ));
    }
    if cfg.buffer.wal_dir.is_empty() {
        return Err(LoadError::Validation("buffer.wal_dir must not be empty".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_config() {
        let yaml = r#"
server: https://localhost:8443
collect:
  interval_seconds: 10
  metrics:
    cpu: true
    mem: true
    disk: false
buffer:
  wal_dir: /tmp/wal
security: {}
"#;
        let cfg = load_from_str(yaml).unwrap();
        assert_eq!(cfg.server, "https://localhost:8443");
        assert!(!cfg.collect.metrics.disk);
    }

    #[test]
    fn empty_server_rejected() {
        let yaml = r#"
server: ""
collect:
  interval_seconds: 10
  metrics: {}
buffer:
  wal_dir: /tmp/wal
security: {}
"#;
        let err = load_from_str(yaml).unwrap_err();
        assert!(err.to_string().contains("server URL"));
    }

    #[test]
    fn zero_interval_rejected() {
        let yaml = r#"
server: https://localhost
collect:
  interval_seconds: 0
  metrics: {}
buffer:
  wal_dir: /tmp/wal
security: {}
"#;
        let err = load_from_str(yaml).unwrap_err();
        assert!(err.to_string().contains("interval_seconds"));
    }

    #[test]
    fn load_from_file_works() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(
            &path,
            "server: https://s\ncollect:\n  interval_seconds: 5\n  metrics: {}\nbuffer:\n  wal_dir: /tmp/w\nsecurity: {}\n",
        )
        .unwrap();
        let cfg = load_from_file(&path).unwrap();
        assert_eq!(cfg.server, "https://s");
    }
}
