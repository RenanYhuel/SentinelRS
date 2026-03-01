CREATE TABLE IF NOT EXISTS agents (
    agent_id   TEXT PRIMARY KEY,
    hw_id      TEXT NOT NULL UNIQUE,
    secret     BYTEA NOT NULL,
    key_id     TEXT NOT NULL,
    agent_version TEXT NOT NULL DEFAULT '',
    registered_at_ms BIGINT NOT NULL DEFAULT 0,
    deprecated_keys JSONB NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_agents_hw_id ON agents (hw_id);
