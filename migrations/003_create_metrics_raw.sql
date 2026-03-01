CREATE TABLE IF NOT EXISTS metrics_raw (
    time        TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
    agent_id    TEXT             NOT NULL,
    batch_id    TEXT             NOT NULL,
    payload     JSONB            NOT NULL
);

SELECT create_hypertable(
    'metrics_raw',
    'time',
    if_not_exists => TRUE,
    chunk_time_interval => INTERVAL '1 day'
);

CREATE INDEX IF NOT EXISTS idx_metrics_raw_agent
    ON metrics_raw (agent_id, time DESC);

CREATE INDEX IF NOT EXISTS idx_metrics_raw_batch
    ON metrics_raw (batch_id);
