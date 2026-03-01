pub mod exposition;
pub mod worker_metrics;

#[cfg(test)]
mod tests {
    use super::exposition::render_prometheus;
    use super::worker_metrics::WorkerMetrics;
    use std::time::Instant;

    #[test]
    fn counters_increment() {
        let m = WorkerMetrics::new();
        m.inc_batches_processed();
        m.inc_batches_processed();
        m.inc_batches_errors();
        assert_eq!(m.batches_processed_val(), 2);
        assert_eq!(m.batches_errors_val(), 1);
    }

    #[test]
    fn latency_recording() {
        let m = WorkerMetrics::new();
        let start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(1));
        m.record_processing_latency(start);
        let (sum, count) = m.processing_latency_vals();
        assert!(sum > 0);
        assert_eq!(count, 1);
    }

    #[test]
    fn prometheus_output_contains_metric_names() {
        let m = WorkerMetrics::new();
        m.inc_batches_processed();
        m.add_rows_inserted(42);
        let output = render_prometheus(&m);
        assert!(output.contains("sentinel_worker_batches_processed_total 1"));
        assert!(output.contains("sentinel_worker_rows_inserted_total 42"));
        assert!(output.contains("# TYPE sentinel_worker_db_latency_us summary"));
    }
}
