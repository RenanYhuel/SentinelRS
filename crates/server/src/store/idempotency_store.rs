use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct IdempotencyStore {
    seen: Arc<DashMap<String, i64>>,
}

impl IdempotencyStore {
    pub fn new() -> Self {
        Self {
            seen: Arc::new(DashMap::new()),
        }
    }

    pub fn is_duplicate(&self, batch_id: &str) -> bool {
        self.seen.contains_key(batch_id)
    }

    pub fn mark_processed(&self, batch_id: String, received_at_ms: i64) {
        self.seen.insert(batch_id, received_at_ms);
    }

    pub fn count(&self) -> usize {
        self.seen.len()
    }

    pub fn evict_older_than(&self, cutoff_ms: i64) {
        self.seen.retain(|_, ts| *ts >= cutoff_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_and_detect_duplicate() {
        let store = IdempotencyStore::new();
        assert!(!store.is_duplicate("b-1"));
        store.mark_processed("b-1".into(), 1000);
        assert!(store.is_duplicate("b-1"));
    }

    #[test]
    fn evict_removes_old() {
        let store = IdempotencyStore::new();
        store.mark_processed("old".into(), 500);
        store.mark_processed("new".into(), 1500);
        store.evict_older_than(1000);
        assert!(!store.is_duplicate("old"));
        assert!(store.is_duplicate("new"));
    }
}
