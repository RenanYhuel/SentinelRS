pub mod exposition;
pub mod pipeline_stats;
pub mod worker_metrics;

#[cfg(test)]
mod tests {
    use super::exposition::render_prometheus;
    use super::worker_metrics::WorkerMetrics;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Instant;

    fn test_pool() -> sqlx::PgPool {
        PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://fake@localhost/fake")
            .unwrap()
    }

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

    #[tokio::test]
    async fn prometheus_output_contains_metric_names() {
        let m = WorkerMetrics::new();
        m.inc_batches_processed();
        m.add_rows_inserted(42);
        let pool = test_pool();
        let output = render_prometheus(&m, "test-worker", &pool);
        assert!(output.contains("sentinel_worker_batches_processed_total"));
        assert!(output.contains("sentinel_worker_rows_inserted_total"));
        assert!(output.contains("# TYPE sentinel_worker_db_latency_us summary"));
        assert!(output.contains("sentinel_worker_pool_size"));
        assert!(output.contains("sentinel_worker_pool_idle"));
    }
}
