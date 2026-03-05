use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;

const TOKEN_BYTES: usize = 32;

pub fn generate_bootstrap_token() -> String {
    let mut buf = [0u8; TOKEN_BYTES];
    rand::thread_rng().fill(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
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
