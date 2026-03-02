use std::time::Duration;

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub database_url: String,
    pub max_db_connections: u32,
    pub nats_url: String,
    pub batch_size: usize,
    pub api_addr: String,
    pub consumer_group: ConsumerGroupConfig,
    pub backpressure: BackpressureConfig,
    pub registry: RegistryConfig,
}

#[derive(Debug, Clone)]
pub struct ConsumerGroupConfig {
    pub group_name: String,
    pub max_ack_pending: i64,
    pub ack_wait: Duration,
    pub max_deliver: i64,
    pub idle_heartbeat: Duration,
}

#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_reset: Duration,
    pub max_concurrent_batches: usize,
}

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub enabled: bool,
    pub bucket: String,
    pub heartbeat_interval: Duration,
    pub ttl: Duration,
}

impl Default for ConsumerGroupConfig {
    fn default() -> Self {
        Self {
            group_name: "sentinel-workers".into(),
            max_ack_pending: 1000,
            ack_wait: Duration::from_secs(30),
            max_deliver: 5,
            idle_heartbeat: Duration::from_secs(5),
        }
    }
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            circuit_breaker_threshold: 5,
            circuit_breaker_reset: Duration::from_secs(15),
            max_concurrent_batches: 100,
        }
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bucket: "sentinel-worker-registry".into(),
            heartbeat_interval: Duration::from_secs(10),
            ttl: Duration::from_secs(30),
        }
    }
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        let consumer_group = ConsumerGroupConfig {
            group_name: env_or("CONSUMER_GROUP", "sentinel-workers"),
            max_ack_pending: env_parse("MAX_ACK_PENDING", 1000),
            ack_wait: Duration::from_secs(env_parse("ACK_WAIT_SECS", 30)),
            max_deliver: env_parse("MAX_DELIVER", 5),
            ..Default::default()
        };

        let backpressure = BackpressureConfig {
            circuit_breaker_threshold: env_parse("CB_THRESHOLD", 5),
            circuit_breaker_reset: Duration::from_secs(env_parse("CB_RESET_SECS", 15)),
            max_concurrent_batches: env_parse("MAX_CONCURRENT_BATCHES", 100),
        };

        let registry = RegistryConfig {
            enabled: env_parse("REGISTRY_ENABLED", true),
            heartbeat_interval: Duration::from_secs(env_parse("REGISTRY_HEARTBEAT_SECS", 10)),
            ttl: Duration::from_secs(env_parse("REGISTRY_TTL_SECS", 30)),
            ..Default::default()
        };

        Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            max_db_connections: env_parse("MAX_DB_CONNECTIONS", 10),
            nats_url: env_or("NATS_URL", "nats://127.0.0.1:4222"),
            batch_size: env_parse("BATCH_SIZE", 50),
            api_addr: env_or("WORKER_API_ADDR", "0.0.0.0:9090"),
            consumer_group,
            backpressure,
            registry,
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.into())
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
        let cg = ConsumerGroupConfig::default();
        assert_eq!(cg.group_name, "sentinel-workers");
        assert_eq!(cg.max_ack_pending, 1000);
        assert_eq!(cg.max_deliver, 5);

        let bp = BackpressureConfig::default();
        assert_eq!(bp.circuit_breaker_threshold, 5);
        assert_eq!(bp.max_concurrent_batches, 100);

        let reg = RegistryConfig::default();
        assert!(reg.enabled);
        assert_eq!(reg.bucket, "sentinel-worker-registry");
    }
}
