CREATE OR REPLACE VIEW metrics AS
SELECT * FROM metrics_time;

CREATE OR REPLACE VIEW v_agents AS
SELECT
    agent_id AS id,
    agent_id,
    hw_id,
    status,
    agent_version,
    last_seen,
    to_timestamp(registered_at_ms / 1000.0) AS registered_at
FROM agents;
