CREATE TABLE IF NOT EXISTS alert_rules (
    id              TEXT        PRIMARY KEY,
    name            TEXT        NOT NULL,
    agent_pattern   TEXT        NOT NULL DEFAULT '*',
    metric_name     TEXT        NOT NULL,
    condition       TEXT        NOT NULL,
    threshold       DOUBLE PRECISION NOT NULL,
    for_duration_ms BIGINT      NOT NULL DEFAULT 0,
    severity        TEXT        NOT NULL DEFAULT 'warning',
    annotations     JSONB       NOT NULL DEFAULT '{}',
    enabled         BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alert_rules_enabled
    ON alert_rules (enabled);

CREATE INDEX IF NOT EXISTS idx_alert_rules_metric
    ON alert_rules (metric_name);
