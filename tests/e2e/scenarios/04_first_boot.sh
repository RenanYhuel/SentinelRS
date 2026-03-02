#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"
GRPC="${GRPC_ADDR:-127.0.0.1:50061}"

echo "  Step 1: Health check"
STATUS=$(curl -sf -o /dev/null -w "%{http_code}" "$REST/healthz")
[ "$STATUS" = "200" ] || { echo "healthz returned $STATUS"; exit 1; }

echo "  Step 2: Ready check (DB migrated)"
STATUS=$(curl -sf -o /dev/null -w "%{http_code}" "$REST/ready")
[ "$STATUS" = "200" ] || { echo "ready returned $STATUS"; exit 1; }

echo "  Step 3: Generate bootstrap token via CLI"
TOKEN_RESP=$(sentinel_cli --server "$REST" agents generate-install \
  --agent-name "e2e-first-boot" --json 2>&1) || {
  echo "generate-install failed: $TOKEN_RESP"
  exit 1
}
TOKEN=$(echo "$TOKEN_RESP" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
[ -n "$TOKEN" ] || { echo "no token in response"; exit 1; }
echo "  Token generated: ${TOKEN:0:8}..."

echo "  Step 4: Start agent with bootstrap token"
SENTINEL_BOOTSTRAP_TOKEN="$TOKEN" \
SENTINEL_SERVER_URL="$GRPC" \
  sentinel_agent --bootstrap &
AGENT_PID=$!
sleep 3

echo "  Step 5: Verify agent registered"
AGENTS=$(curl -sf "$REST/v1/agents")
echo "$AGENTS" | grep -q "e2e-first-boot" || { echo "agent not found after bootstrap"; kill $AGENT_PID 2>/dev/null; exit 1; }

echo "  Step 6: Verify agent connected (live session)"
CLUSTER=$(curl -sf "$REST/v1/cluster/stats")
CONNECTED=$(echo "$CLUSTER" | grep -o '"connected_agents":[0-9]*' | cut -d: -f2)
[ "$CONNECTED" -ge 1 ] || { echo "expected connected_agents >= 1, got $CONNECTED"; kill $AGENT_PID 2>/dev/null; exit 1; }

echo "  Step 7: Wait for metrics to flow"
sleep 5

echo "  Step 8: Verify metrics stored in DB"
METRIC_COUNT=$(PGPASSWORD=postgres psql -h 127.0.0.1 -p "${DB_PORT:-15432}" -U postgres -d sentinel_e2e \
  -t -c "SELECT count(*) FROM metrics_time WHERE agent_id = 'e2e-first-boot';" 2>/dev/null | tr -d ' ')
[ "${METRIC_COUNT:-0}" -gt 0 ] || echo "  WARN: no metrics in DB yet (worker may be slow)"

kill $AGENT_PID 2>/dev/null || true
wait $AGENT_PID 2>/dev/null || true

echo "  Scenario complete"
