pub fn build_agent_config_yaml(agent_id: &str, secret_b64: &str, server_url: &str) -> String {
    format!(
        r#"agent_id: "{agent_id}"
server: "{server_url}"
secret: "{secret_b64}"
collect:
  interval_seconds: 10
  metrics:
    cpu: true
    mem: true
    disk: true
buffer:
  wal_dir: /var/lib/sentinel/wal
  segment_size_mb: 16
  max_retention_days: 7
security:
  key_store: auto
  rotation_check_interval_hours: 24
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_contains_fields() {
        let yaml = build_agent_config_yaml("agent-42", "c2VjcmV0", "https://srv:50051");
        assert!(yaml.contains("agent_id: \"agent-42\""));
        assert!(yaml.contains("secret: \"c2VjcmV0\""));
        assert!(yaml.contains("server: \"https://srv:50051\""));
        assert!(yaml.contains("interval_seconds: 10"));
    }
}
