use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub agent_pattern: String,
    pub metric_name: String,
    pub condition: Condition,
    pub threshold: f64,
    pub for_duration_ms: i64,
    pub severity: Severity,
    pub annotations: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Condition {
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Equal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl Condition {
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            Self::GreaterThan => value > threshold,
            Self::LessThan => value < threshold,
            Self::GreaterOrEqual => value >= threshold,
            Self::LessOrEqual => value <= threshold,
            Self::Equal => (value - threshold).abs() < f64::EPSILON,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_greater_than() {
        assert!(Condition::GreaterThan.evaluate(10.0, 5.0));
        assert!(!Condition::GreaterThan.evaluate(5.0, 10.0));
    }

    #[test]
    fn condition_less_than() {
        assert!(Condition::LessThan.evaluate(1.0, 5.0));
        assert!(!Condition::LessThan.evaluate(10.0, 5.0));
    }

    #[test]
    fn condition_equal() {
        assert!(Condition::Equal.evaluate(5.0, 5.0));
        assert!(!Condition::Equal.evaluate(5.1, 5.0));
    }

    #[test]
    fn condition_boundaries() {
        assert!(Condition::GreaterOrEqual.evaluate(5.0, 5.0));
        assert!(Condition::LessOrEqual.evaluate(5.0, 5.0));
    }
}
