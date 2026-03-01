pub trait KeyStore: Send + Sync {
    fn store(&self, agent_id: &str, secret: &[u8]) -> Result<(), KeyStoreError>;
    fn load(&self, agent_id: &str) -> Result<Vec<u8>, KeyStoreError>;
    fn delete(&self, agent_id: &str) -> Result<(), KeyStoreError>;
}

#[derive(Debug)]
pub enum KeyStoreError {
    NotFound,
    Io(String),
    Crypto(String),
}

impl std::fmt::Display for KeyStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "key not found"),
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Crypto(e) => write!(f, "crypto: {e}"),
        }
    }
}

impl std::error::Error for KeyStoreError {}
