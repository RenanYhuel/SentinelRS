use sentinel_common::crypto::sign_data;

pub fn sign_payload(secret: &[u8], payload: &[u8]) -> String {
    sign_data(secret, payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_common::crypto::verify_signature;

    #[test]
    fn sign_and_verify() {
        let secret = b"webhook-secret";
        let payload = b"{\"alert\":\"test\"}";
        let sig = sign_payload(secret, payload);
        assert!(verify_signature(secret, payload, &sig));
    }
}
