use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecatedKey {
    pub key_id: String,
    pub secret: Vec<u8>,
    pub deprecated_at_ms: i64,
}
