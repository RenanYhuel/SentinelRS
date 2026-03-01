use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use std::path::Path;

const NONCE_LEN: usize = 12;

pub trait KeyStore: Send + Sync {
    fn store(&self, key_id: &str, secret: &[u8]) -> Result<(), KeyStoreError>;
    fn load(&self, key_id: &str) -> Result<Vec<u8>, KeyStoreError>;
    fn delete(&self, key_id: &str) -> Result<(), KeyStoreError>;
}

#[derive(Debug)]
pub enum KeyStoreError {
    NotFound,
    Io(std::io::Error),
    Crypto(String),
}

impl std::fmt::Display for KeyStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "key not found"),
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Crypto(msg) => write!(f, "crypto: {msg}"),
        }
    }
}

impl std::error::Error for KeyStoreError {}

impl From<std::io::Error> for KeyStoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub struct EncryptedFileStore {
    dir: std::path::PathBuf,
    master_key: [u8; 32],
}

impl EncryptedFileStore {
    pub fn new(dir: &Path, master_key: [u8; 32]) -> Self {
        Self {
            dir: dir.to_path_buf(),
            master_key,
        }
    }

    fn path_for(&self, key_id: &str) -> std::path::PathBuf {
        self.dir.join(format!("{key_id}.enc"))
    }
}

impl KeyStore for EncryptedFileStore {
    fn store(&self, key_id: &str, secret: &[u8]) -> Result<(), KeyStoreError> {
        std::fs::create_dir_all(&self.dir)?;
        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| KeyStoreError::Crypto(e.to_string()))?;

        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, secret)
            .map_err(|e| KeyStoreError::Crypto(e.to_string()))?;

        let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);

        std::fs::write(self.path_for(key_id), blob)?;
        Ok(())
    }

    fn load(&self, key_id: &str) -> Result<Vec<u8>, KeyStoreError> {
        let path = self.path_for(key_id);
        if !path.exists() {
            return Err(KeyStoreError::NotFound);
        }

        let blob = std::fs::read(path)?;
        if blob.len() < NONCE_LEN {
            return Err(KeyStoreError::Crypto("blob too short".into()));
        }

        let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| KeyStoreError::Crypto(e.to_string()))?;
        let nonce = Nonce::from_slice(nonce_bytes);

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| KeyStoreError::Crypto(e.to_string()))
    }

    fn delete(&self, key_id: &str) -> Result<(), KeyStoreError> {
        let path = self.path_for(key_id);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> (tempfile::TempDir, EncryptedFileStore) {
        let dir = tempfile::tempdir().unwrap();
        let key = [0xABu8; 32];
        let store = EncryptedFileStore::new(dir.path(), key);
        (dir, store)
    }

    #[test]
    fn store_and_load() {
        let (_dir, store) = test_store();
        let secret = b"super-secret-hmac-key";
        store.store("agent-1", secret).unwrap();
        let loaded = store.load("agent-1").unwrap();
        assert_eq!(loaded, secret);
    }

    #[test]
    fn load_missing_returns_not_found() {
        let (_dir, store) = test_store();
        let err = store.load("nonexistent").unwrap_err();
        assert!(matches!(err, KeyStoreError::NotFound));
    }

    #[test]
    fn delete_removes_key() {
        let (_dir, store) = test_store();
        store.store("temp", b"data").unwrap();
        store.delete("temp").unwrap();
        assert!(matches!(store.load("temp"), Err(KeyStoreError::NotFound)));
    }

    #[test]
    fn wrong_key_fails_decrypt() {
        let dir = tempfile::tempdir().unwrap();
        let store_a = EncryptedFileStore::new(dir.path(), [0x01u8; 32]);
        store_a.store("k", b"secret").unwrap();

        let store_b = EncryptedFileStore::new(dir.path(), [0x02u8; 32]);
        let err = store_b.load("k").unwrap_err();
        assert!(matches!(err, KeyStoreError::Crypto(_)));
    }
}
