#[cfg(test)]
mod tests {
    use crate::output::{build_table, print_json, OutputMode};

    #[test]
    fn output_mode_json() {
        let mode = OutputMode::Json;
        assert_eq!(mode, OutputMode::Json);
    }

    #[test]
    fn output_mode_human() {
        let mode = OutputMode::Human;
        assert_eq!(mode, OutputMode::Human);
    }

    #[test]
    fn print_json_valid() {
        let val = serde_json::json!({"key": "value"});
        assert!(print_json(&val).is_ok());
    }

    #[test]
    fn build_table_creates_headers() {
        let table = build_table(&["A", "B", "C"]);
        let rendered = table.to_string();
        assert!(rendered.contains("A"));
        assert!(rendered.contains("B"));
        assert!(rendered.contains("C"));
    }

    #[test]
    fn build_table_with_rows() {
        let mut table = build_table(&["Name", "Value"]);
        table.add_row(vec!["foo", "bar"]);
        let rendered = table.to_string();
        assert!(rendered.contains("foo"));
        assert!(rendered.contains("bar"));
    }
}
