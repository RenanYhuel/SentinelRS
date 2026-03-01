use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AgentConfig {
    pub agent_id: Option<String>,
    pub server: String,
    pub collect: CollectConfig,
    #[serde(default = "default_plugins_dir")]
    pub plugins_dir: String,
    pub buffer: BufferConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CollectConfig {
    pub interval_seconds: u64,
    pub metrics: MetricsToggle,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MetricsToggle {
    #[serde(default = "yes")]
    pub cpu: bool,
    #[serde(default = "yes")]
    pub mem: bool,
    #[serde(default = "yes")]
    pub disk: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct BufferConfig {
    pub wal_dir: String,
    #[serde(default = "default_segment_size")]
    pub segment_size_mb: u64,
    #[serde(default = "default_retention_days")]
    pub max_retention_days: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SecurityConfig {
    #[serde(default = "default_key_store")]
    pub key_store: String,
    #[serde(default = "default_rotation_hours")]
    pub rotation_check_interval_hours: u64,
}

fn default_plugins_dir() -> String {
    "/var/lib/sentinel/plugins".to_string()
}

fn default_segment_size() -> u64 {
    16
}

fn default_retention_days() -> u64 {
    7
}

fn default_key_store() -> String {
    "auto".to_string()
}

fn default_rotation_hours() -> u64 {
    24
}

fn yes() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_full() {
        let yaml = r#"
agent_id: null
server: https://sentinel.example.com:8443
collect:
  interval_seconds: 10
  metrics:
    cpu: true
    mem: true
    disk: true
plugins_dir: /var/lib/sentinel/plugins
buffer:
  wal_dir: /var/lib/sentinel/wal
  segment_size_mb: 16
  max_retention_days: 7
security:
  key_store: auto
  rotation_check_interval_hours: 24
"#;
        let cfg: AgentConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.server, "https://sentinel.example.com:8443");
        assert!(cfg.agent_id.is_none());
        assert_eq!(cfg.collect.interval_seconds, 10);
        assert!(cfg.collect.metrics.cpu);
        assert_eq!(cfg.buffer.segment_size_mb, 16);
        assert_eq!(cfg.security.key_store, "auto");
    }

    #[test]
    fn defaults_applied() {
        let yaml = r#"
server: https://localhost
collect:
  interval_seconds: 5
  metrics: {}
buffer:
  wal_dir: /tmp/wal
security: {}
"#;
        let cfg: AgentConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(cfg.collect.metrics.cpu);
        assert!(cfg.collect.metrics.mem);
        assert!(cfg.collect.metrics.disk);
        assert_eq!(cfg.buffer.segment_size_mb, 16);
        assert_eq!(cfg.buffer.max_retention_days, 7);
        assert_eq!(cfg.security.key_store, "auto");
        assert_eq!(cfg.security.rotation_check_interval_hours, 24);
    }
}
