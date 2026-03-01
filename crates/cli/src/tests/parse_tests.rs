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
        assert_eq!(opts.output_mode(), crate::output::OutputMode::Human);
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
        assert!(matches!(opts.cmd, crate::cmd::Commands::Health(_)));
    }

    #[test]
    fn parse_status() {
        let opts = parse(&["status"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::Status(_)));
    }

    #[test]
    fn parse_tail_logs() {
        let opts = parse(&["tail-logs", "--lines", "50"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::TailLogs(_)));
    }

    #[test]
    fn parse_force_send() {
        let opts = parse(&["force-send", "--limit", "10"]);
        assert!(matches!(opts.cmd, crate::cmd::Commands::ForceSend(_)));
    }

    #[test]
    fn parse_combined_flags() {
        let opts = parse(&["--json", "--server", "http://my:8080", "health"]);
        assert!(opts.json);
        assert_eq!(opts.server.as_deref(), Some("http://my:8080"));
    }
}
