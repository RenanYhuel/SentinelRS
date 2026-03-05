use super::worker_metrics::WorkerMetrics;
use sqlx::PgPool;
use std::sync::Arc;

pub fn render_prometheus(m: &Arc<WorkerMetrics>, worker_id: &str, pool: &PgPool) -> String {
    let mut out = String::with_capacity(2048);

    write_counter(
        &mut out,
        "sentinel_worker_batches_processed_total",
        m.batches_processed_val(),
        worker_id,
    );
    write_counter(
        &mut out,
        "sentinel_worker_batches_errors_total",
        m.batches_errors_val(),
        worker_id,
    );
    write_counter(
        &mut out,
        "sentinel_worker_messages_acked_total",
        m.messages_acked_val(),
        worker_id,
    );
    write_counter(
        &mut out,
        "sentinel_worker_messages_nacked_total",
        m.messages_nacked_val(),
        worker_id,
    );
    write_counter(
        &mut out,
        "sentinel_worker_rows_inserted_total",
        m.rows_inserted_val(),
        worker_id,
    );
    write_counter(
        &mut out,
        "sentinel_worker_alerts_fired_total",
        m.alerts_fired_val(),
        worker_id,
    );
    write_counter(
        &mut out,
        "sentinel_worker_notifications_sent_total",
        m.notifications_sent_val(),
        worker_id,
    );
    write_counter(
        &mut out,
        "sentinel_worker_notifications_failed_total",
        m.notifications_failed_val(),
        worker_id,
    );

    let (sum, count) = m.processing_latency_vals();
    write_summary(
        &mut out,
        "sentinel_worker_processing_latency_us",
        sum,
        count,
        worker_id,
    );

    let (sum, count) = m.db_latency_vals();
    write_summary(
        &mut out,
        "sentinel_worker_db_latency_us",
        sum,
        count,
        worker_id,
    );

    write_gauge(
        &mut out,
        "sentinel_worker_pool_size",
        u64::from(pool.size()),
        worker_id,
    );
    write_gauge(
        &mut out,
        "sentinel_worker_pool_idle",
        pool.num_idle() as u64,
        worker_id,
    );

    out
}

fn write_counter(out: &mut String, name: &str, val: u64, worker_id: &str) {
    use std::fmt::Write;
    let _ = writeln!(out, "# TYPE {name} counter");
    let _ = writeln!(out, "{name}{{worker=\"{worker_id}\"}} {val}");
}

fn write_gauge(out: &mut String, name: &str, val: u64, worker_id: &str) {
    use std::fmt::Write;
    let _ = writeln!(out, "# TYPE {name} gauge");
    let _ = writeln!(out, "{name}{{worker=\"{worker_id}\"}} {val}");
}

fn write_summary(out: &mut String, name: &str, sum: u64, count: u64, worker_id: &str) {
    use std::fmt::Write;
    let _ = writeln!(out, "# TYPE {name} summary");
    let _ = writeln!(out, "{name}_sum{{worker=\"{worker_id}\"}} {sum}");
    let _ = writeln!(out, "{name}_count{{worker=\"{worker_id}\"}} {count}");
}
