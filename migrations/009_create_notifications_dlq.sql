CREATE TABLE IF NOT EXISTS notifications_dlq (
    id           TEXT             PRIMARY KEY,
    alert_id     TEXT             NOT NULL,
    notifier     TEXT             NOT NULL,
    payload      JSONB            NOT NULL,
    error        TEXT             NOT NULL,
    attempts     INTEGER          NOT NULL DEFAULT 1,
    last_attempt TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
    created_at   TIMESTAMPTZ      NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notifications_dlq_alert
    ON notifications_dlq (alert_id);

CREATE INDEX IF NOT EXISTS idx_notifications_dlq_notifier
    ON notifications_dlq (notifier, created_at DESC);
