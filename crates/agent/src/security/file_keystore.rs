use std::fs;
use std::path::PathBuf;

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;

use super::keystore::{KeyStore, KeyStoreError};

pub struct EncryptedFileKeyStore {
    dir: PathBuf,
    master_key: [u8; 32],
}

impl EncryptedFileKeyStore {
    pub fn new(dir: PathBuf, master_key: [u8; 32]) -> Result<Self, KeyStoreError> {
        fs::create_dir_all(&dir).map_err(|e| KeyStoreError::Io(e.to_string()))?;
        Ok(Self { dir, master_key })
    }

    fn path_for(&self, agent_id: &str) -> PathBuf {
        self.dir.join(format!("{agent_id}.key"))
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, KeyStoreError> {
        let cipher =
            Aes256Gcm::new_from_slice(&self.master_key).map_err(|e| KeyStoreError::Crypto(e.to_string()))?;
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| KeyStoreError::Crypto(e.to_string()))?;
        let mut out = nonce_bytes.to_vec();
        out.extend(ciphertext);
        Ok(out)
    }

    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, KeyStoreError> {
        if data.len() < 12 {
            return Err(KeyStoreError::Crypto("data too short".into()));
        }
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let cipher =
            Aes256Gcm::new_from_slice(&self.master_key).map_err(|e| KeyStoreError::Crypto(e.to_string()))?;
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| KeyStoreError::Crypto(e.to_string()))
    }
}

impl KeyStore for EncryptedFileKeyStore {
    fn store(&self, agent_id: &str, secret: &[u8]) -> Result<(), KeyStoreError> {
        let encrypted = self.encrypt(secret)?;
        fs::write(self.path_for(agent_id), encrypted).map_err(|e| KeyStoreError::Io(e.to_string()))
    }

    fn load(&self, agent_id: &str) -> Result<Vec<u8>, KeyStoreError> {
        let path = self.path_for(agent_id);
        if !path.exists() {
            return Err(KeyStoreError::NotFound);
        }
        let data = fs::read(&path).map_err(|e| KeyStoreError::Io(e.to_string()))?;
        self.decrypt(&data)
    }

    fn delete(&self, agent_id: &str) -> Result<(), KeyStoreError> {
        let path = self.path_for(agent_id);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| KeyStoreError::Io(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn mk_store(dir: &Path) -> EncryptedFileKeyStore {
        let key = [0xABu8; 32];
        EncryptedFileKeyStore::new(dir.to_path_buf(), key).unwrap()
    }

    #[test]
    fn roundtrip_store_load() {
        let dir = tempfile::tempdir().unwrap();
        let ks = mk_store(dir.path());
        ks.store("agent-1", b"my-secret").unwrap();
        let loaded = ks.load("agent-1").unwrap();
        assert_eq!(loaded, b"my-secret");
    }

    #[test]
    fn load_missing_returns_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let ks = mk_store(dir.path());
        match ks.load("nope") {
            Err(KeyStoreError::NotFound) => {}
            other => panic!("expected NotFound, got: {other:?}"),
        }
    }

    #[test]
    fn delete_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        let ks = mk_store(dir.path());
        ks.store("agent-1", b"secret").unwrap();
        ks.delete("agent-1").unwrap();
        assert!(matches!(ks.load("agent-1"), Err(KeyStoreError::NotFound)));
    }

    #[test]
    fn tampered_data_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let ks = mk_store(dir.path());
        ks.store("agent-1", b"secret").unwrap();
        let path = ks.path_for("agent-1");
        let mut data = fs::read(&path).unwrap();
        if let Some(b) = data.last_mut() {
            *b ^= 0xFF;
        }
        fs::write(&path, &data).unwrap();
        assert!(ks.load("agent-1").is_err());
    }
}
