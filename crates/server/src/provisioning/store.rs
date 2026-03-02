use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;

#[derive(Debug, Clone)]
pub struct BootstrapToken {
    pub token: String,
    pub agent_name: String,
    pub labels: Vec<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub consumed: bool,
}

#[derive(Clone)]
pub struct TokenStore {
    tokens: Arc<DashMap<String, BootstrapToken>>,
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenStore {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(DashMap::new()),
        }
    }

    pub fn insert(&self, entry: BootstrapToken) {
        self.tokens.insert(entry.token.clone(), entry);
    }

    pub fn get(&self, token: &str) -> Option<BootstrapToken> {
        self.tokens.get(token).map(|e| e.value().clone())
    }

    pub fn consume(&self, token: &str) -> bool {
        if let Some(mut entry) = self.tokens.get_mut(token) {
            if entry.consumed {
                return false;
            }
            entry.consumed = true;
            return true;
        }
        false
    }

    pub fn remove(&self, token: &str) {
        self.tokens.remove(token);
    }

    pub fn purge_expired(&self) {
        let now = Utc::now();
        self.tokens.retain(|_, v| v.expires_at > now);
    }

    pub fn count_active(&self) -> usize {
        let now = Utc::now();
        self.tokens
            .iter()
            .filter(|e| !e.consumed && e.expires_at > now)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_token(token: &str, ttl_minutes: i64) -> BootstrapToken {
        let now = Utc::now();
        BootstrapToken {
            token: token.into(),
            agent_name: "test-agent".into(),
            labels: vec!["env:test".into()],
            created_by: "admin".into(),
            created_at: now,
            expires_at: now + Duration::minutes(ttl_minutes),
            consumed: false,
        }
    }

    #[test]
    fn insert_and_get() {
        let store = TokenStore::new();
        store.insert(sample_token("tok-1", 60));
        assert!(store.get("tok-1").is_some());
        assert!(store.get("nope").is_none());
    }

    #[test]
    fn consume_once() {
        let store = TokenStore::new();
        store.insert(sample_token("tok-2", 60));
        assert!(store.consume("tok-2"));
        assert!(!store.consume("tok-2"));
    }

    #[test]
    fn purge_expired() {
        let store = TokenStore::new();
        store.insert(sample_token("fresh", 60));
        store.insert(sample_token("stale", -1));
        store.purge_expired();
        assert!(store.get("fresh").is_some());
        assert!(store.get("stale").is_none());
    }
}
