pub struct MigrationFile {
    pub filename: &'static str,
    pub sql: &'static str,
}

pub fn load_all() -> Vec<MigrationFile> {
    vec![
        MigrationFile {
            filename: "000_migration_tracking.sql",
            sql: include_str!("../../../../migrations/000_migration_tracking.sql"),
        },
        MigrationFile {
            filename: "001_create_extensions.sql",
            sql: include_str!("../../../../migrations/001_create_extensions.sql"),
        },
        MigrationFile {
            filename: "002_create_metrics_time.sql",
            sql: include_str!("../../../../migrations/002_create_metrics_time.sql"),
        },
        MigrationFile {
            filename: "003_create_metrics_raw.sql",
            sql: include_str!("../../../../migrations/003_create_metrics_raw.sql"),
        },
        MigrationFile {
            filename: "004_create_alerts.sql",
            sql: include_str!("../../../../migrations/004_create_alerts.sql"),
        },
        MigrationFile {
            filename: "005_retention_policies.sql",
            sql: include_str!("../../../../migrations/005_retention_policies.sql"),
        },
        MigrationFile {
            filename: "006_continuous_aggregates.sql",
            sql: include_str!("../../../../migrations/006_continuous_aggregates.sql"),
        },
        MigrationFile {
            filename: "007_dashboard_views.sql",
            sql: include_str!("../../../../migrations/007_dashboard_views.sql"),
        },
        MigrationFile {
            filename: "008_create_alert_rules.sql",
            sql: include_str!("../../../../migrations/008_create_alert_rules.sql"),
        },
        MigrationFile {
            filename: "009_create_notifications_dlq.sql",
            sql: include_str!("../../../../migrations/009_create_notifications_dlq.sql"),
        },
        MigrationFile {
            filename: "010_create_agents.sql",
            sql: include_str!("../../../../migrations/010_create_agents.sql"),
        },
        MigrationFile {
            filename: "011_add_agents_last_seen.sql",
            sql: include_str!("../../../../migrations/011_add_agents_last_seen.sql"),
        },
        MigrationFile {
            filename: "012_create_notifier_configs.sql",
            sql: include_str!("../../../../migrations/012_create_notifier_configs.sql"),
        },
        MigrationFile {
            filename: "013_create_bootstrap_tokens.sql",
            sql: include_str!("../../../../migrations/013_create_bootstrap_tokens.sql"),
        },
        MigrationFile {
            filename: "014_add_agent_status_column.sql",
            sql: include_str!("../../../../migrations/014_add_agent_status_column.sql"),
        },
        MigrationFile {
            filename: "015_add_rule_notifier_ids.sql",
            sql: include_str!("../../../../migrations/015_add_rule_notifier_ids.sql"),
        },
        MigrationFile {
            filename: "016_notification_history.sql",
            sql: include_str!("../../../../migrations/016_notification_history.sql"),
        },
        MigrationFile {
            filename: "017_metrics_5m_aggregate.sql",
            sql: include_str!("../../../../migrations/017_metrics_5m_aggregate.sql"),
        },
    ]
}
