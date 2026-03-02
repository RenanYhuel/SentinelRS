use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

const TOKEN_BYTES: usize = 32;

pub fn generate_bootstrap_token() -> String {
    let bytes: Vec<u8> = (0..TOKEN_BYTES).map(|_| rand_byte()).collect();
    URL_SAFE_NO_PAD.encode(bytes)
}

fn rand_byte() -> u8 {
    uuid::Uuid::new_v4().as_bytes()[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_non_empty() {
        let t = generate_bootstrap_token();
        assert!(!t.is_empty());
    }

    #[test]
    fn tokens_are_unique() {
        let a = generate_bootstrap_token();
        let b = generate_bootstrap_token();
        assert_ne!(a, b);
    }
}
