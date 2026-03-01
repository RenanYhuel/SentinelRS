use uuid::Uuid;

pub fn generate() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_uuid_v4() {
        let id = generate();
        let parsed = Uuid::parse_str(&id).expect("must be valid UUID");
        assert_eq!(parsed.get_version_num(), 4);
    }

    #[test]
    fn generates_unique_ids() {
        let a = generate();
        let b = generate();
        assert_ne!(a, b);
    }
}
