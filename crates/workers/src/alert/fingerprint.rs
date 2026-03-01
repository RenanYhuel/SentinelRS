use std::hash::{Hash, Hasher};

pub fn fingerprint(rule_id: &str, agent_id: &str, metric_name: &str) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    rule_id.hash(&mut hasher);
    agent_id.hash(&mut hasher);
    metric_name.hash(&mut hasher);
    hasher.finish()
}

pub fn fingerprint_string(rule_id: &str, agent_id: &str, metric_name: &str) -> String {
    format!("{:016x}", fingerprint(rule_id, agent_id, metric_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let a = fingerprint("r1", "agent-1", "cpu");
        let b = fingerprint("r1", "agent-1", "cpu");
        assert_eq!(a, b);
    }

    #[test]
    fn different_inputs_different_fingerprint() {
        let a = fingerprint("r1", "agent-1", "cpu");
        let b = fingerprint("r1", "agent-2", "cpu");
        assert_ne!(a, b);
    }

    #[test]
    fn string_is_hex() {
        let s = fingerprint_string("r1", "a1", "m1");
        assert_eq!(s.len(), 16);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
