use std::sync::Arc;
use super::server_metrics::ServerMetrics;

pub fn render_prometheus(m: &Arc<ServerMetrics>) -> String {
    let mut out = String::with_capacity(1024);

    write_counter(&mut out, "sentinel_server_grpc_requests_total", m.grpc_requests_total());
    write_counter(&mut out, "sentinel_server_grpc_errors_total", m.grpc_errors_total());
    write_counter(&mut out, "sentinel_server_rest_requests_total", m.rest_requests_total());
    write_counter(&mut out, "sentinel_server_registrations_total", m.registrations_total());
    write_counter(&mut out, "sentinel_server_pushes_accepted_total", m.pushes_accepted_total());
    write_counter(&mut out, "sentinel_server_pushes_rejected_total", m.pushes_rejected_total());
    write_counter(&mut out, "sentinel_server_heartbeats_total", m.heartbeats_total());
    write_counter(&mut out, "sentinel_server_key_rotations_total", m.key_rotations_total());
    write_counter(&mut out, "sentinel_server_broker_publish_errors_total", m.broker_publish_errors_total());

    let (sum, count) = m.grpc_latency_vals();
    write_summary(&mut out, "sentinel_server_grpc_latency_us", sum, count);

    out
}

fn write_counter(out: &mut String, name: &str, val: u64) {
    use std::fmt::Write;
    let _ = writeln!(out, "# TYPE {name} counter");
    let _ = writeln!(out, "{name} {val}");
}

fn write_summary(out: &mut String, name: &str, sum: u64, count: u64) {
    use std::fmt::Write;
    let _ = writeln!(out, "# TYPE {name} summary");
    let _ = writeln!(out, "{name}_sum {sum}");
    let _ = writeln!(out, "{name}_count {count}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prometheus_output() {
        let m = ServerMetrics::new();
        m.inc_grpc_requests();
        m.inc_pushes_accepted();
        let output = render_prometheus(&m);
        assert!(output.contains("sentinel_server_grpc_requests_total 1"));
        assert!(output.contains("sentinel_server_pushes_accepted_total 1"));
        assert!(output.contains("# TYPE sentinel_server_grpc_latency_us summary"));
    }
}
