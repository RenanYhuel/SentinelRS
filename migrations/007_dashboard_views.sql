CREATE OR REPLACE VIEW v_top_metrics AS
SELECT
    name,
    agent_id,
    COUNT(*)                     AS total_samples,
    MAX(time)                    AS last_seen
FROM metrics_time
GROUP BY name, agent_id
ORDER BY total_samples DESC;

CREATE OR REPLACE VIEW v_recent_values AS
SELECT DISTINCT ON (agent_id, name)
    agent_id,
    name,
    value,
    time
FROM metrics_time
WHERE value IS NOT NULL
ORDER BY agent_id, name, time DESC;

CREATE OR REPLACE VIEW v_active_alerts AS
SELECT *
FROM alerts
WHERE status = 'firing'
ORDER BY created_at DESC;
