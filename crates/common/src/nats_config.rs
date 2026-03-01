pub const STREAM_NAME: &str = "SENTINEL_METRICS";
pub const SUBJECT: &str = "sentinel.metrics.>";
pub const SUBJECT_PREFIX: &str = "sentinel.metrics";
pub const CONSUMER_NAME: &str = "sentinel-workers";

pub fn subject_for_agent(agent_id: &str) -> String {
    format!("{SUBJECT_PREFIX}.{agent_id}")
}

#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub name: String,
    pub subjects: Vec<String>,
    pub max_bytes: i64,
    pub max_age_secs: u64,
    pub retention: RetentionPolicy,
    pub storage: StorageType,
    pub num_replicas: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum RetentionPolicy {
    Limits,
    WorkQueue,
}

#[derive(Debug, Clone, Copy)]
pub enum StorageType {
    File,
    Memory,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            name: STREAM_NAME.into(),
            subjects: vec![SUBJECT.into()],
            max_bytes: 1_073_741_824,
            max_age_secs: 86400 * 7,
            retention: RetentionPolicy::Limits,
            storage: StorageType::File,
            num_replicas: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_for_agent_format() {
        assert_eq!(subject_for_agent("agent-abc"), "sentinel.metrics.agent-abc");
    }

    #[test]
    fn default_stream_config() {
        let cfg = StreamConfig::default();
        assert_eq!(cfg.name, "SENTINEL_METRICS");
        assert_eq!(cfg.max_bytes, 1_073_741_824);
    }
}
