SELECT add_retention_policy('metrics_raw', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_retention_policy('metrics_time', INTERVAL '90 days', if_not_exists => TRUE);
