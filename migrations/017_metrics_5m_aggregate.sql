CREATE MATERIALIZED VIEW IF NOT EXISTS mv_metrics_5m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('5 minutes', time) AS bucket,
    agent_id,
    name,
    AVG(value)   AS avg_value,
    MIN(value)   AS min_value,
    MAX(value)   AS max_value,
    COUNT(*)     AS sample_count
FROM metrics_time
WHERE value IS NOT NULL
GROUP BY bucket, agent_id, name
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mv_metrics_5m',
    start_offset    => INTERVAL '30 minutes',
    end_offset      => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes',
    if_not_exists   => TRUE
);
