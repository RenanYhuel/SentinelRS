use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct BatchDedup {
    seen: Arc<DashMap<String, i64>>,
}

impl BatchDedup {
    pub fn new() -> Self {
        Self {
            seen: Arc::new(DashMap::new()),
        }
    }

    pub fn is_duplicate(&self, batch_id: &str) -> bool {
        self.seen.contains_key(batch_id)
    }

    pub fn mark_processed(&self, batch_id: String) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        self.seen.insert(batch_id, now_ms);
    }

    pub fn evict_older_than(&self, max_age_ms: i64) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        self.seen.retain(|_, ts| now_ms - *ts < max_age_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_and_detect_duplicate() {
        let dedup = BatchDedup::new();
        assert!(!dedup.is_duplicate("b-1"));
        dedup.mark_processed("b-1".into());
        assert!(dedup.is_duplicate("b-1"));
    }

    #[test]
    fn evict_removes_old() {
        let dedup = BatchDedup::new();
        dedup.seen.insert("old".into(), 0);
        dedup.evict_older_than(1);
        assert!(!dedup.is_duplicate("old"));
    }

    #[test]
    fn different_ids_not_duplicate() {
        let dedup = BatchDedup::new();
        dedup.mark_processed("b-1".into());
        assert!(!dedup.is_duplicate("b-2"));
    }
}
