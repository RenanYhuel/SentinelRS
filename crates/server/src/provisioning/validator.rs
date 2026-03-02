use chrono::Utc;

use super::store::{BootstrapToken, TokenStore};

pub enum ValidationResult {
    Valid(BootstrapToken),
    NotFound,
    Expired,
    AlreadyConsumed,
}

pub fn validate_and_consume(store: &TokenStore, token_str: &str) -> ValidationResult {
    let entry = match store.get(token_str) {
        Some(e) => e,
        None => return ValidationResult::NotFound,
    };

    if entry.consumed {
        return ValidationResult::AlreadyConsumed;
    }

    if entry.expires_at <= Utc::now() {
        return ValidationResult::Expired;
    }

    if !store.consume(token_str) {
        return ValidationResult::AlreadyConsumed;
    }

    ValidationResult::Valid(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provisioning::store::BootstrapToken;
    use chrono::Duration;

    fn make_store(token: &str, ttl_min: i64) -> TokenStore {
        let now = Utc::now();
        let store = TokenStore::new();
        store.insert(BootstrapToken {
            token: token.into(),
            agent_name: "test".into(),
            labels: vec![],
            created_by: "admin".into(),
            created_at: now,
            expires_at: now + Duration::minutes(ttl_min),
            consumed: false,
        });
        store
    }

    #[test]
    fn valid_token() {
        let store = make_store("tok", 60);
        assert!(matches!(
            validate_and_consume(&store, "tok"),
            ValidationResult::Valid(_)
        ));
    }

    #[test]
    fn not_found() {
        let store = TokenStore::new();
        assert!(matches!(
            validate_and_consume(&store, "nope"),
            ValidationResult::NotFound
        ));
    }

    #[test]
    fn expired() {
        let store = make_store("tok", -1);
        assert!(matches!(
            validate_and_consume(&store, "tok"),
            ValidationResult::Expired
        ));
    }

    #[test]
    fn double_consume() {
        let store = make_store("tok", 60);
        validate_and_consume(&store, "tok");
        assert!(matches!(
            validate_and_consume(&store, "tok"),
            ValidationResult::AlreadyConsumed
        ));
    }
}
