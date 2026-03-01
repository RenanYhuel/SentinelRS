pub use sentinel_common::crypto::{generate_secret, verify_signature};

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_common::crypto::sign_data;

    #[test]
    fn roundtrip_sign_verify() {
        let secret = b"test-secret";
        let data = b"hello world";
        let sig = sign_data(secret, data);
        assert!(verify_signature(secret, data, &sig));
    }

    #[test]
    fn wrong_signature_rejected() {
        assert!(!verify_signature(b"secret", b"data", "bad-base64!"));
    }

    #[test]
    fn generate_secret_is_16_bytes() {
        assert_eq!(generate_secret().len(), 16);
    }
}
