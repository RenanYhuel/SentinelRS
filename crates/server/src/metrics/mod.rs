pub mod exposition;
pub mod server_metrics;

#[cfg(test)]
mod tests {
    use super::server_metrics::ServerMetrics;
    use super::exposition::render_prometheus;

    #[test]
    fn prometheus_contains_all_counters() {
        let m = ServerMetrics::new();
        m.inc_grpc_requests();
        m.inc_rest_requests();
        m.inc_registrations();
        let output = render_prometheus(&m);
        assert!(output.contains("sentinel_server_grpc_requests_total 1"));
        assert!(output.contains("sentinel_server_rest_requests_total 1"));
        assert!(output.contains("sentinel_server_registrations_total 1"));
        assert!(output.contains("sentinel_server_pushes_accepted_total 0"));
    }
}
