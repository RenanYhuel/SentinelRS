use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_signature(secret: &[u8], data: &[u8], signature_b64: &str) -> bool {
    let Ok(sig_bytes) = STANDARD.decode(signature_b64) else {
        return false;
    };
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(data);
    mac.verify_slice(&sig_bytes).is_ok()
}

pub fn generate_secret() -> Vec<u8> {
    uuid::Uuid::new_v4().as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_sign_verify() {
        let secret = b"test-secret";
        let data = b"hello world";
        let mut mac = HmacSha256::new_from_slice(secret).unwrap();
        mac.update(data);
        let sig = STANDARD.encode(mac.finalize().into_bytes());
        assert!(verify_signature(secret, data, &sig));
    }

    #[test]
    fn wrong_signature_rejected() {
        assert!(!verify_signature(b"secret", b"data", "bad-base64!"));
        assert!(!verify_signature(b"secret", b"data", &STANDARD.encode(b"wrong")));
    }

    #[test]
    fn generate_secret_is_16_bytes() {
        let s = generate_secret();
        assert_eq!(s.len(), 16);
    }
}
