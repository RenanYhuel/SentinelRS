use std::sync::Arc;
use super::worker_metrics::WorkerMetrics;

pub fn render_prometheus(m: &Arc<WorkerMetrics>) -> String {
    let mut out = String::with_capacity(1024);

    write_counter(&mut out, "sentinel_worker_batches_processed_total", m.batches_processed_val());
    write_counter(&mut out, "sentinel_worker_batches_errors_total", m.batches_errors_val());
    write_counter(&mut out, "sentinel_worker_messages_acked_total", m.messages_acked_val());
    write_counter(&mut out, "sentinel_worker_messages_nacked_total", m.messages_nacked_val());
    write_counter(&mut out, "sentinel_worker_rows_inserted_total", m.rows_inserted_val());
    write_counter(&mut out, "sentinel_worker_alerts_fired_total", m.alerts_fired_val());
    write_counter(&mut out, "sentinel_worker_notifications_sent_total", m.notifications_sent_val());
    write_counter(&mut out, "sentinel_worker_notifications_failed_total", m.notifications_failed_val());

    let (sum, count) = m.processing_latency_vals();
    write_summary(&mut out, "sentinel_worker_processing_latency_us", sum, count);

    let (sum, count) = m.db_latency_vals();
    write_summary(&mut out, "sentinel_worker_db_latency_us", sum, count);

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
