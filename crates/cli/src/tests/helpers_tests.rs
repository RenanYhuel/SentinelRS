#[cfg(test)]
mod tests {
    use crate::cmd::helpers;

    #[test]
    fn default_config_path_not_empty() {
        let path = helpers::default_config_path();
        assert!(!path.to_string_lossy().is_empty());
    }

    #[test]
    fn load_config_missing_file() {
        let result = helpers::load_config(Some("/nonexistent/path.yml"));
        assert!(result.is_err());
    }

    #[test]
    fn load_config_from_tempfile() {
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

        let cfg = helpers::load_config(Some(path.to_str().unwrap()));
        assert!(cfg.is_ok());
        let cfg = cfg.unwrap();
        assert_eq!(cfg.server, "http://localhost:50051");
    }

    #[test]
    fn resolve_server_uses_flag() {
        let result = helpers::resolve_server(Some("http://override:8080"), None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "http://override:8080");
    }

    #[test]
    fn resolve_server_falls_back_to_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.yml");
        std::fs::write(
            &path,
            r#"
server: http://fromconfig:50051
collect:
  interval_seconds: 5
  metrics: {}
buffer:
  wal_dir: /tmp/wal
security: {}
"#,
        )
        .unwrap();

        let result = helpers::resolve_server(None, Some(path.to_str().unwrap()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "http://fromconfig:50051");
    }

    #[test]
    fn resolve_rest_url_transforms_grpc() {
        let result = helpers::resolve_rest_url(Some("grpc://localhost:50051"), None);
        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.contains("8080"));
        assert!(url.starts_with("http://"));
    }
}
