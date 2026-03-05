CREATE OR REPLACE VIEW metrics AS
SELECT * FROM metrics_time;

CREATE OR REPLACE VIEW v_agents AS
SELECT
    agent_id AS id,
    agent_id,
    name,
    status,
    labels,
    last_seen,
    created_at
FROM agents;
