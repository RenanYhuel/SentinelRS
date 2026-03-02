use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct WorkerIdentity {
    id: String,
    hostname: String,
    suffix: String,
    started_at: String,
    #[serde(skip)]
    boot_instant: Instant,
}

impl WorkerIdentity {
    pub fn generate() -> Arc<Self> {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".into());

        let suffix = &uuid::Uuid::new_v4().to_string()[..8];
        let id = format!("{hostname}-{suffix}");

        Arc::new(Self {
            id,
            hostname,
            suffix: suffix.to_string(),
            started_at: Utc::now().to_rfc3339(),
            boot_instant: Instant::now(),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    pub fn uptime_secs(&self) -> u64 {
        self.boot_instant.elapsed().as_secs()
    }

    pub fn uptime_human(&self) -> String {
        let s = self.uptime_secs();
        if s < 60 {
            format!("{s}s")
        } else if s < 3600 {
            format!("{}m {}s", s / 60, s % 60)
        } else {
            format!("{}h {}m", s / 3600, (s % 3600) / 60)
        }
    }

    pub fn started_at(&self) -> &str {
        &self.started_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_unique() {
        let a = WorkerIdentity::generate();
        let b = WorkerIdentity::generate();
        assert_ne!(a.id(), b.id());
        assert!(!a.id().is_empty());
        assert!(!a.hostname().is_empty());
    }

    #[test]
    fn uptime_starts_at_zero() {
        let id = WorkerIdentity::generate();
        assert!(id.uptime_secs() < 2);
    }

    #[test]
    fn uptime_human_format() {
        let id = WorkerIdentity::generate();
        let h = id.uptime_human();
        assert!(h.contains('s'));
    }
}
