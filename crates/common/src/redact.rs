use std::fmt;

const MIN_REVEAL_LEN: usize = 8;
const PREFIX_LEN: usize = 4;
const SUFFIX_LEN: usize = 4;

pub fn mask_token(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    if value.len() <= MIN_REVEAL_LEN {
        return "***".into();
    }
    format!(
        "{}***{}",
        &value[..PREFIX_LEN],
        &value[value.len() - SUFFIX_LEN..]
    )
}

pub fn mask_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let user_start = scheme_end + 3;
            return format!("{}***@{}", &url[..user_start], &url[at_pos + 1..]);
        }
    }
    url.to_string()
}

pub struct RedactedSecret<'a>(pub &'a str);

impl fmt::Display for RedactedSecret<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", mask_token(self.0))
    }
}

impl fmt::Debug for RedactedSecret<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", mask_token(self.0))
    }
}

pub struct RedactedBytes<'a>(pub &'a [u8]);

impl fmt::Display for RedactedBytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.len() <= 4 {
            write!(f, "[{} bytes]", self.0.len())
        } else {
            write!(
                f,
                "[{} bytes {:02x}{:02x}…]",
                self.0.len(),
                self.0[0],
                self.0[1]
            )
        }
    }
}

impl fmt::Debug for RedactedBytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_short_token() {
        assert_eq!(mask_token("abc"), "***");
        assert_eq!(mask_token("12345678"), "***");
    }

    #[test]
    fn mask_long_token() {
        assert_eq!(mask_token("abcdefghij"), "abcd***ghij");
    }

    #[test]
    fn mask_empty() {
        assert_eq!(mask_token(""), "");
    }

    #[test]
    fn mask_url_with_password() {
        assert_eq!(
            mask_url("postgres://user:pass@host:5432/db"),
            "postgres://***@host:5432/db"
        );
    }

    #[test]
    fn mask_url_without_auth() {
        assert_eq!(mask_url("http://localhost:8080"), "http://localhost:8080");
    }

    #[test]
    fn redacted_secret_display() {
        let s = RedactedSecret("super-secret-token-12345");
        assert_eq!(format!("{s}"), "supe***2345");
    }

    #[test]
    fn redacted_bytes_display() {
        let b = RedactedBytes(b"hello world");
        let out = format!("{b}");
        assert!(out.contains("11 bytes"));
        assert!(out.contains("68"));
    }

    #[test]
    fn redacted_bytes_short() {
        let b = RedactedBytes(b"hi");
        assert_eq!(format!("{b}"), "[2 bytes]");
    }
}
