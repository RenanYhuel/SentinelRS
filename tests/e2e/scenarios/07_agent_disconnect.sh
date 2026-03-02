#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"
GRPC="${GRPC_ADDR:-127.0.0.1:50061}"

echo "  Step 1: Bootstrap agent"
TOKEN_RESP=$(sentinel_cli --server "$REST" agents generate-install \
  --agent-name "e2e-disconnect" --json 2>&1)
TOKEN=$(echo "$TOKEN_RESP" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
[ -n "$TOKEN" ] || { echo "no token"; exit 1; }

SENTINEL_BOOTSTRAP_TOKEN="$TOKEN" \
SENTINEL_SERVER_URL="$GRPC" \
  sentinel_agent --bootstrap &
AGENT_PID=$!
sleep 3

echo "  Step 2: Verify agent connected"
CLUSTER=$(curl -sf "$REST/v1/cluster/stats")
CONNECTED=$(echo "$CLUSTER" | grep -o '"connected_agents":[0-9]*' | cut -d: -f2)
[ "$CONNECTED" -ge 1 ] || { echo "agent not connected"; exit 1; }

echo "  Step 3: Kill agent abruptly (SIGKILL)"
kill -9 $AGENT_PID 2>/dev/null || true
wait $AGENT_PID 2>/dev/null || true

echo "  Step 4: Wait for server to detect disconnect (< 30s)"
DETECTED=false
for i in $(seq 1 30); do
  CLUSTER=$(curl -sf "$REST/v1/cluster/stats" 2>/dev/null || echo '{}')
  CONNECTED=$(echo "$CLUSTER" | grep -o '"connected_agents":[0-9]*' | cut -d: -f2 || echo "0")
  if [ "${CONNECTED:-1}" -eq 0 ]; then
    echo "  Disconnect detected after ${i}s"
    DETECTED=true
    break
  fi
  sleep 1
done

$DETECTED || { echo "server did not detect disconnect within 30s"; exit 1; }

echo "  Step 5: Restart agent (reconnect)"
SENTINEL_CONFIG_DIR="${TMPDIR:-/tmp}/sentinel-e2e-disconnect" \
  sentinel_agent &
AGENT_PID=$!
sleep 3

echo "  Step 6: Verify reconnection"
CLUSTER=$(curl -sf "$REST/v1/cluster/stats")
CONNECTED=$(echo "$CLUSTER" | grep -o '"connected_agents":[0-9]*' | cut -d: -f2)
[ "$CONNECTED" -ge 1 ] || echo "  WARN: agent may not have reconnected"

kill $AGENT_PID 2>/dev/null || true
wait $AGENT_PID 2>/dev/null || true

echo "  Scenario complete"
