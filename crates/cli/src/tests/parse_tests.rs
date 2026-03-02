#[cfg(test)]
mod tests {
    use crate::Opts;
    use clap::Parser;

    fn parse(args: &[&str]) -> Opts {
        let mut full = vec!["sentinel"];
        full.extend_from_slice(args);
        Opts::parse_from(full)
    }

    #[test]
    fn parse_version() {
        let opts = parse(&["version"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Version));
    }

    #[test]
    fn parse_json_flag() {
        let opts = parse(&["--json", "version"]);
        assert!(opts.json);
        assert_eq!(opts.output_mode(), crate::output::OutputMode::Json);
    }

    #[test]
    fn parse_human_flag_default() {
        let opts = parse(&["version"]);
        assert!(!opts.json);
    }

    #[test]
    fn parse_server_flag() {
        let opts = parse(&["--server", "http://localhost:9090", "version"]);
        assert_eq!(opts.server.as_deref(), Some("http://localhost:9090"));
    }

    #[test]
    fn parse_config_flag() {
        let opts = parse(&["--config", "/tmp/agent.yml", "version"]);
        assert_eq!(opts.config.as_deref(), Some("/tmp/agent.yml"));
    }

    #[test]
    fn parse_init() {
        let opts = parse(&["init"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Init));
    }

    #[test]
    fn parse_doctor() {
        let opts = parse(&["doctor"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Doctor));
    }

    #[test]
    fn parse_completions() {
        let opts = parse(&["completions", "bash"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Completions { .. }));
    }

    #[test]
    fn parse_register() {
        let opts = parse(&["register", "--hw-id", "abc123"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Register(_)));
    }

    #[test]
    fn parse_config_show() {
        let opts = parse(&["config", "show"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Config(_)));
    }

    #[test]
    fn parse_wal_stats() {
        let opts = parse(&["wal", "stats"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Wal(_)));
    }

    #[test]
    fn parse_wal_inspect_with_limit() {
        let opts = parse(&["wal", "inspect", "--limit", "50"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Wal(_)));
    }

    #[test]
    fn parse_wal_compact() {
        let opts = parse(&["wal", "compact", "--force", "--yes"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Wal(_)));
    }

    #[test]
    fn parse_wal_meta() {
        let opts = parse(&["wal", "meta"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Wal(_)));
    }

    #[test]
    fn parse_agents_list() {
        let opts = parse(&["agents", "list"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Agents(_)));
    }

    #[test]
    fn parse_agents_get() {
        let opts = parse(&["agents", "get", "agent-123"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Agents(_)));
    }

    #[test]
    fn parse_agents_live() {
        let opts = parse(&["agents", "live", "agent-123"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Agents(_)));
    }

    #[test]
    fn parse_agents_delete() {
        let opts = parse(&["agents", "delete", "agent-123", "--yes"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Agents(_)));
    }

    #[test]
    fn parse_agents_generate_install() {
        let opts = parse(&["agents", "generate-install"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Agents(_)));
    }

    #[test]
    fn parse_cluster_status() {
        let opts = parse(&["cluster", "status"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Cluster(_)));
    }

    #[test]
    fn parse_cluster_agents() {
        let opts = parse(&["cluster", "agents"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Cluster(_)));
    }

    #[test]
    fn parse_cluster_watch() {
        let opts = parse(&["cluster", "watch"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Cluster(_)));
    }

    #[test]
    fn parse_rules_list() {
        let opts = parse(&["rules", "list"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Rules(_)));
    }

    #[test]
    fn parse_rules_create() {
        let opts = parse(&["rules", "create", "--data", r#"{"name":"test"}"#]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Rules(_)));
    }

    #[test]
    fn parse_rules_delete() {
        let opts = parse(&["rules", "delete", "rule-1"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Rules(_)));
    }

    #[test]
    fn parse_notifiers_test() {
        let opts = parse(&[
            "notifiers",
            "test",
            "--type",
            "webhook",
            "--target",
            "http://example.com",
        ]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Notifiers(_)));
    }

    #[test]
    fn parse_key_rotate() {
        let opts = parse(&["key", "rotate", "--key-id", "k1", "--secret", "c2VjcmV0"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Key(_)));
    }

    #[test]
    fn parse_key_list() {
        let opts = parse(&["key", "list"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Key(_)));
    }

    #[test]
    fn parse_health() {
        let opts = parse(&["health"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Health));
    }

    #[test]
    fn parse_status() {
        let opts = parse(&["status"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Status));
    }

    #[test]
    fn parse_force_send() {
        let opts = parse(&["force-send", "--limit", "10"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::ForceSend(_)));
    }

    #[test]
    fn parse_metrics_show() {
        let opts = parse(&["metrics", "show"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Metrics(_)));
    }

    #[test]
    fn parse_metrics_live() {
        let opts = parse(&["metrics", "live", "--interval", "5"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Metrics(_)));
    }

    #[test]
    fn parse_config_edit() {
        let opts = parse(&["config", "edit"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Config(_)));
    }

    #[test]
    fn parse_config_path() {
        let opts = parse(&["config", "path"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Config(_)));
    }

    #[test]
    fn parse_config_reset() {
        let opts = parse(&["config", "reset"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Config(_)));
    }

    #[test]
    fn parse_combined_flags() {
        let opts = parse(&["--json", "--server", "http://my:8080", "health"]);
        assert!(opts.json);
        assert_eq!(opts.server.as_deref(), Some("http://my:8080"));
    }
}
