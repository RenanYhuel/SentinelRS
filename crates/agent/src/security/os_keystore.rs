use std::collections::HashMap;
use std::sync::Mutex;

use super::keystore::{KeyStore, KeyStoreError};

pub struct OsKeyStore {
    inner: Mutex<HashMap<String, Vec<u8>>>,
}

impl OsKeyStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl KeyStore for OsKeyStore {
    fn store(&self, agent_id: &str, secret: &[u8]) -> Result<(), KeyStoreError> {
        self.inner
            .lock()
            .map_err(|e| KeyStoreError::Io(e.to_string()))?
            .insert(agent_id.to_string(), secret.to_vec());
        Ok(())
    }

    fn load(&self, agent_id: &str) -> Result<Vec<u8>, KeyStoreError> {
        self.inner
            .lock()
            .map_err(|e| KeyStoreError::Io(e.to_string()))?
            .get(agent_id)
            .cloned()
            .ok_or(KeyStoreError::NotFound)
    }

    fn delete(&self, agent_id: &str) -> Result<(), KeyStoreError> {
        self.inner
            .lock()
            .map_err(|e| KeyStoreError::Io(e.to_string()))?
            .remove(agent_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let ks = OsKeyStore::new();
        ks.store("agent-1", b"secret").unwrap();
        assert_eq!(ks.load("agent-1").unwrap(), b"secret");
    }

    #[test]
    fn missing_returns_not_found() {
        let ks = OsKeyStore::new();
        assert!(matches!(ks.load("nope"), Err(KeyStoreError::NotFound)));
    }

    #[test]
    fn delete_removes() {
        let ks = OsKeyStore::new();
        ks.store("agent-1", b"secret").unwrap();
        ks.delete("agent-1").unwrap();
        assert!(matches!(ks.load("agent-1"), Err(KeyStoreError::NotFound)));
    }
}
