//! sentinel_common
//!
//! Shared types, constants and proto files for SentinelRS.
//!
//! NOTE: This crate currently contains proto examples and placeholders only.

pub const CRATE_NAME: &str = "sentinel_common";

#[cfg(test)]
mod tests {
    #[test]
    fn it_has_name() {
        assert_eq!(super::CRATE_NAME, "sentinel_common");
    }
}
