use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn create_token(secret: &[u8], subject: &str, expires_at_ms: i64) -> String {
    let header = STANDARD.encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
    let claims = serde_json::json!({
        "sub": subject,
        "exp": expires_at_ms / 1000,
    });
    let payload = STANDARD.encode(claims.to_string().as_bytes());
    let unsigned = format!("{header}.{payload}");
    let mut mac = HmacSha256::new_from_slice(secret).expect("valid key");
    mac.update(unsigned.as_bytes());
    let sig = STANDARD.encode(mac.finalize().into_bytes());
    format!("{unsigned}.{sig}")
}

pub fn validate_token(secret: &[u8], token: &str) -> Result<String, TokenError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(TokenError::Malformed);
    }

    let unsigned = format!("{}.{}", parts[0], parts[1]);
    let mut mac = HmacSha256::new_from_slice(secret).expect("valid key");
    mac.update(unsigned.as_bytes());
    let expected_sig = STANDARD.encode(mac.finalize().into_bytes());

    if expected_sig != parts[2] {
        return Err(TokenError::InvalidSignature);
    }

    let payload_bytes = STANDARD
        .decode(parts[1])
        .map_err(|_| TokenError::Malformed)?;
    let claims: serde_json::Value =
        serde_json::from_slice(&payload_bytes).map_err(|_| TokenError::Malformed)?;

    let sub = claims["sub"]
        .as_str()
        .ok_or(TokenError::Malformed)?
        .to_string();

    if let Some(exp) = claims["exp"].as_i64() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        if now > exp {
            return Err(TokenError::Expired);
        }
    }

    Ok(sub)
}

#[derive(Debug, PartialEq)]
pub enum TokenError {
    Malformed,
    InvalidSignature,
    Expired,
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => write!(f, "malformed token"),
            Self::InvalidSignature => write!(f, "invalid signature"),
            Self::Expired => write!(f, "token expired"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_validate() {
        let secret = b"jwt-secret";
        let far_future = i64::MAX / 2;
        let token = create_token(secret, "admin", far_future);
        let sub = validate_token(secret, &token).unwrap();
        assert_eq!(sub, "admin");
    }

    #[test]
    fn wrong_secret_rejected() {
        let token = create_token(b"secret-a", "admin", i64::MAX / 2);
        let err = validate_token(b"secret-b", &token).unwrap_err();
        assert_eq!(err, TokenError::InvalidSignature);
    }

    #[test]
    fn expired_token_rejected() {
        let token = create_token(b"secret", "admin", 0);
        let err = validate_token(b"secret", &token).unwrap_err();
        assert_eq!(err, TokenError::Expired);
    }

    #[test]
    fn malformed_token() {
        let err = validate_token(b"secret", "not.a.valid.token").unwrap_err();
        assert_eq!(err, TokenError::Malformed);
    }
}
