use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct HmacSigner {
    secret: Vec<u8>,
}

impl HmacSigner {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
        }
    }

    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts any key length");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    pub fn sign_base64(&self, data: &[u8]) -> String {
        STANDARD.encode(self.sign(data))
    }

    pub fn verify(&self, data: &[u8], signature: &[u8]) -> bool {
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts any key length");
        mac.update(data);
        mac.verify_slice(signature).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify() {
        let signer = HmacSigner::new(b"my-secret-key");
        let data = b"batch payload bytes";
        let sig = signer.sign(data);
        assert!(signer.verify(data, &sig));
    }

    #[test]
    fn reject_tampered_data() {
        let signer = HmacSigner::new(b"my-secret-key");
        let sig = signer.sign(b"original");
        assert!(!signer.verify(b"tampered", &sig));
    }

    #[test]
    fn reject_wrong_key() {
        let signer_a = HmacSigner::new(b"key-a");
        let signer_b = HmacSigner::new(b"key-b");
        let sig = signer_a.sign(b"data");
        assert!(!signer_b.verify(b"data", &sig));
    }

    #[test]
    fn base64_encoding() {
        let signer = HmacSigner::new(b"secret");
        let encoded = signer.sign_base64(b"payload");
        let decoded = STANDARD.decode(&encoded).unwrap();
        assert!(signer.verify(b"payload", &decoded));
    }
}
