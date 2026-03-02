#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"
GRPC="${GRPC_ADDR:-127.0.0.1:50061}"

echo "  Step 1: Start 1 additional worker"
SENTINEL_NATS_URL="${NATS_URL}" \
SENTINEL_DATABASE_URL="${DATABASE_URL}" \
  sentinel_workers &
WORKER2_PID=$!
sleep 2

echo "  Step 2: Start another worker (total 3 including main)"
SENTINEL_NATS_URL="${NATS_URL}" \
SENTINEL_DATABASE_URL="${DATABASE_URL}" \
  sentinel_workers &
WORKER3_PID=$!
sleep 2

echo "  Step 3: Bootstrap agent and send metrics"
TOKEN_RESP=$(sentinel_cli --server "$REST" agents generate-install \
  --agent-name "e2e-scale" --json 2>&1)
TOKEN=$(echo "$TOKEN_RESP" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

SENTINEL_BOOTSTRAP_TOKEN="$TOKEN" \
SENTINEL_SERVER_URL="$GRPC" \
  sentinel_agent --bootstrap &
AGENT_PID=$!
sleep 5

echo "  Step 4: Verify metrics flowing"
AGENTS=$(curl -sf "$REST/v1/agents")
echo "$AGENTS" | grep -q "e2e-scale" || { echo "agent not registered"; exit 1; }

echo "  Step 5: Scale down to 1 worker"
kill $WORKER3_PID 2>/dev/null || true
wait $WORKER3_PID 2>/dev/null || true
kill $WORKER2_PID 2>/dev/null || true
wait $WORKER2_PID 2>/dev/null || true
sleep 2

echo "  Step 6: Verify agent still connected"
CLUSTER=$(curl -sf "$REST/v1/cluster/stats")
CONNECTED=$(echo "$CLUSTER" | grep -o '"connected_agents":[0-9]*' | cut -d: -f2)
[ "$CONNECTED" -ge 1 ] || { echo "agent disconnected after scale-down"; exit 1; }

kill $AGENT_PID 2>/dev/null || true
wait $AGENT_PID 2>/dev/null || true

echo "  Scenario complete"
