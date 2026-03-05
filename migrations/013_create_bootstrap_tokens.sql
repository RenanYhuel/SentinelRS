CREATE TABLE IF NOT EXISTS bootstrap_tokens (
    token       TEXT        PRIMARY KEY,
    agent_name  TEXT        NOT NULL,
    labels      JSONB       NOT NULL DEFAULT '[]',
    created_by  TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL,
    consumed    BOOLEAN     NOT NULL DEFAULT FALSE,
    consumed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_bootstrap_tokens_active
    ON bootstrap_tokens (consumed, expires_at)
    WHERE consumed = FALSE;
