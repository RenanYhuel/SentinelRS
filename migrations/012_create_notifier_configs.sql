CREATE TABLE IF NOT EXISTS notifier_configs (
    id          TEXT        PRIMARY KEY,
    name        TEXT        NOT NULL UNIQUE,
    ntype       TEXT        NOT NULL,
    config      JSONB       NOT NULL DEFAULT '{}',
    enabled     BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notifier_configs_ntype
    ON notifier_configs (ntype);
