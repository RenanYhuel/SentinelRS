CREATE TABLE IF NOT EXISTS metrics_time (
    time        TIMESTAMPTZ      NOT NULL,
    agent_id    TEXT             NOT NULL,
    name        TEXT             NOT NULL,
    labels      JSONB            NOT NULL DEFAULT '{}',
    metric_type TEXT             NOT NULL DEFAULT 'gauge',
    value       DOUBLE PRECISION,
    histogram_boundaries DOUBLE PRECISION[],
    histogram_counts     BIGINT[],
    histogram_count      BIGINT,
    histogram_sum        DOUBLE PRECISION
);

SELECT create_hypertable(
    'metrics_time',
    'time',
    if_not_exists => TRUE,
    chunk_time_interval => INTERVAL '1 day'
);

CREATE INDEX IF NOT EXISTS idx_metrics_time_agent
    ON metrics_time (agent_id, time DESC);

CREATE INDEX IF NOT EXISTS idx_metrics_time_name
    ON metrics_time (name, time DESC);

CREATE INDEX IF NOT EXISTS idx_metrics_time_labels
    ON metrics_time USING GIN (labels);
