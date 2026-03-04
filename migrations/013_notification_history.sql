CREATE TABLE IF NOT EXISTS notification_history (
    id          TEXT PRIMARY KEY,
    alert_id    TEXT NOT NULL,
    notifier_id TEXT NOT NULL,
    ntype       TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'sent',
    error       TEXT,
    attempts    INTEGER NOT NULL DEFAULT 1,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    sent_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_notification_history_alert
    ON notification_history (alert_id);
CREATE INDEX IF NOT EXISTS idx_notification_history_notifier
    ON notification_history (notifier_id);
CREATE INDEX IF NOT EXISTS idx_notification_history_status
    ON notification_history (status);
CREATE INDEX IF NOT EXISTS idx_notification_history_sent_at
    ON notification_history (sent_at DESC);
