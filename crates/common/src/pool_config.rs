use std::time::Duration;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub idle_timeout: Duration,
    pub acquire_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            idle_timeout: Duration::from_secs(300),
            acquire_timeout: Duration::from_secs(5),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

impl PoolConfig {
    pub fn from_env() -> Self {
        Self {
            max_connections: env_parse("MAX_DB_CONNECTIONS", 10),
            min_connections: env_parse("MIN_DB_CONNECTIONS", 1),
            idle_timeout: Duration::from_secs(env_parse("DB_IDLE_TIMEOUT_SECS", 300)),
            acquire_timeout: Duration::from_secs(env_parse("DB_ACQUIRE_TIMEOUT_SECS", 5)),
            max_lifetime: Duration::from_secs(env_parse("DB_MAX_LIFETIME_SECS", 1800)),
        }
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let cfg = PoolConfig::default();
        assert_eq!(cfg.max_connections, 10);
        assert_eq!(cfg.min_connections, 1);
        assert_eq!(cfg.idle_timeout, Duration::from_secs(300));
        assert_eq!(cfg.acquire_timeout, Duration::from_secs(5));
        assert_eq!(cfg.max_lifetime, Duration::from_secs(1800));
    }
}
