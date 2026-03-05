pub fn db_unreachable(url: &str, error: &dyn std::fmt::Display) -> String {
    format!(
        "Database unreachable at {url}: {error}. \
         Check DATABASE_URL, ensure TimescaleDB is running, and verify network connectivity."
    )
}

pub fn nats_unreachable(url: &str, error: &dyn std::fmt::Display) -> String {
    format!(
        "NATS unreachable at {url}: {error}. \
         Check NATS_URL, ensure the NATS server is running with JetStream enabled."
    )
}

pub fn tls_load_failed(cert_path: &str, error: &dyn std::fmt::Display) -> String {
    format!(
        "TLS certificate load failed ({cert_path}): {error}. \
         Check file permissions, ensure PEM format, and verify cert/key pairing."
    )
}

pub fn config_missing(key: &str) -> String {
    format!(
        "Required configuration '{key}' is missing. \
         Set it via environment variable or config file. \
         Run `sentinel doctor` to diagnose configuration issues."
    )
}

pub fn port_in_use(addr: &str, error: &dyn std::fmt::Display) -> String {
    format!(
        "Cannot bind to {addr}: {error}. \
         Another process may be using this port. \
         Check with `netstat` or change the listen address."
    )
}

pub fn wal_corrupt(path: &str, error: &dyn std::fmt::Display) -> String {
    format!(
        "WAL corruption detected in {path}: {error}. \
         Run `sentinel agent repair-wal` to attempt recovery, \
         or delete the WAL directory to start fresh (data loss)."
    )
}

pub fn auth_failed(agent_id: &str, reason: &str) -> String {
    format!(
        "Authentication failed for agent '{agent_id}': {reason}. \
         Verify the agent secret matches the server configuration, \
         check key expiration, and ensure clocks are synchronized."
    )
}

pub fn migration_failed(file: &str, error: &dyn std::fmt::Display) -> String {
    format!(
        "Migration '{file}' failed: {error}. \
         Check database permissions and schema state. \
         Run `sentinel server migrate --status` for details."
    )
}

pub fn broker_publish_failed(batch_id: &str, error: &dyn std::fmt::Display) -> String {
    format!(
        "Failed to publish batch {batch_id} to NATS: {error}. \
         Check NATS connectivity and JetStream stream health. \
         The batch will be retried automatically."
    )
}
