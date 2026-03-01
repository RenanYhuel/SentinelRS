CREATE MATERIALIZED VIEW IF NOT EXISTS mv_metrics_1h
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', time)  AS bucket,
    agent_id,
    name,
    AVG(value)                   AS avg_value,
    MIN(value)                   AS min_value,
    MAX(value)                   AS max_value,
    COUNT(*)                     AS sample_count
FROM metrics_time
WHERE value IS NOT NULL
GROUP BY bucket, agent_id, name
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mv_metrics_1h',
    start_offset  => INTERVAL '3 hours',
    end_offset    => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour',
    if_not_exists => TRUE
);
