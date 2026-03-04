#[cfg(test)]
mod tests {
    use crate::cmd::wal::helpers;

    #[test]
    fn default_agent_config_path_not_empty() {
        let path = helpers::default_agent_config_path();
        assert!(!path.to_string_lossy().is_empty());
    }

    #[test]
    fn load_agent_config_missing_file() {
        let result = helpers::load_agent_config(Some("/nonexistent/path.yml"));
        assert!(result.is_err());
    }

    #[test]
    fn load_agent_config_from_tempfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.yml");
        std::fs::write(
            &path,
            r#"
server: http://localhost:50051
collect:
  interval_seconds: 10
  metrics: {}
buffer:
  wal_dir: /tmp/wal
security: {}
"#,
        )
        .unwrap();

        let cfg = helpers::load_agent_config(Some(path.to_str().unwrap()));
        assert!(cfg.is_ok());
        let cfg = cfg.unwrap();
        assert_eq!(cfg.server, "http://localhost:50051");
    }

    #[test]
    fn format_bytes_human_readable() {
        assert_eq!(helpers::format_bytes(0), "0 B");
        assert_eq!(helpers::format_bytes(1023), "1023 B");
        assert_eq!(helpers::format_bytes(1024), "1.0 KB");
        assert_eq!(helpers::format_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn store_config_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut cfg = crate::store::config::CliConfig::default();
        cfg.server.url = "http://test:8080".into();
        cfg.defaults.output_format = "json".into();

        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        std::fs::write(&path, &toml_str).unwrap();

        let loaded: crate::store::config::CliConfig =
            toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.server.url, "http://test:8080");
        assert_eq!(loaded.defaults.output_format, "json");
    }

    #[test]
    fn client_normalize_grpc_to_http() {
        let normalized = crate::client::normalize("grpc://localhost:50051");
        assert!(!normalized.contains("grpc://"));
        assert!(normalized.contains("http"));
    }
}
