pub fn generate_trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
