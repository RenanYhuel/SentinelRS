CREATE TABLE IF NOT EXISTS alerts (
    id            TEXT             PRIMARY KEY,
    fingerprint   TEXT             NOT NULL,
    rule_id       TEXT             NOT NULL,
    rule_name     TEXT             NOT NULL,
    agent_id      TEXT             NOT NULL,
    metric_name   TEXT             NOT NULL,
    severity      TEXT             NOT NULL,
    status        TEXT             NOT NULL,
    value         DOUBLE PRECISION NOT NULL,
    threshold     DOUBLE PRECISION NOT NULL,
    fired_at      TIMESTAMPTZ      NOT NULL,
    resolved_at   TIMESTAMPTZ,
    annotations   JSONB            NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ      NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alerts_agent
    ON alerts (agent_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_alerts_rule
    ON alerts (rule_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_alerts_fingerprint
    ON alerts (fingerprint);

CREATE INDEX IF NOT EXISTS idx_alerts_status
    ON alerts (status, created_at DESC);
