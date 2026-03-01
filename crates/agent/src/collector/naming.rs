pub fn normalize_name(raw: &str) -> String {
    raw.chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '.' | '_' => c,
            'A'..='Z' => c.to_ascii_lowercase(),
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_and_replaces_invalid_chars() {
        assert_eq!(normalize_name("CPU-Core#0"), "cpu_core_0");
    }

    #[test]
    fn keeps_dots_and_underscores() {
        assert_eq!(normalize_name("cpu.core_0.usage"), "cpu.core_0.usage");
    }
}
