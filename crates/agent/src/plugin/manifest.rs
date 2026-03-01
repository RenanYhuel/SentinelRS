use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub entry_fn: String,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub resource_limits: ResourceLimits,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    HttpGet,
    ReadFile,
    MetricBuilder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceLimits {
    #[serde(default = "default_memory_mb")]
    pub max_memory_mb: u64,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_max_metrics")]
    pub max_metrics: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: default_memory_mb(),
            timeout_ms: default_timeout_ms(),
            max_metrics: default_max_metrics(),
        }
    }
}

fn default_memory_mb() -> u64 {
    64
}

fn default_timeout_ms() -> u64 {
    5000
}

fn default_max_metrics() -> u64 {
    1000
}

impl PluginManifest {
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_full_manifest() {
        let yaml = r#"
name: nginx_status
version: "1.0.0"
entry_fn: collect
capabilities:
  - http_get
resource_limits:
  max_memory_mb: 32
  timeout_ms: 3000
  max_metrics: 500
metadata:
  author: sentinel-team
"#;
        let m = PluginManifest::from_yaml(yaml).unwrap();
        assert_eq!(m.name, "nginx_status");
        assert_eq!(m.version, "1.0.0");
        assert_eq!(m.entry_fn, "collect");
        assert!(m.has_capability(&Capability::HttpGet));
        assert_eq!(m.resource_limits.max_memory_mb, 32);
        assert_eq!(m.resource_limits.timeout_ms, 3000);
        assert_eq!(m.metadata.get("author").unwrap(), "sentinel-team");
    }

    #[test]
    fn defaults_applied() {
        let yaml = r#"
name: basic
version: "0.1.0"
entry_fn: run
"#;
        let m = PluginManifest::from_yaml(yaml).unwrap();
        assert_eq!(m.resource_limits.max_memory_mb, 64);
        assert_eq!(m.resource_limits.timeout_ms, 5000);
        assert_eq!(m.resource_limits.max_metrics, 1000);
        assert!(m.capabilities.is_empty());
    }

    #[test]
    fn roundtrip_yaml() {
        let m = PluginManifest {
            name: "test".into(),
            version: "1.0.0".into(),
            entry_fn: "collect".into(),
            capabilities: vec![Capability::HttpGet, Capability::ReadFile],
            resource_limits: ResourceLimits::default(),
            metadata: HashMap::new(),
        };
        let yaml = m.to_yaml().unwrap();
        let m2 = PluginManifest::from_yaml(&yaml).unwrap();
        assert_eq!(m, m2);
    }
}
