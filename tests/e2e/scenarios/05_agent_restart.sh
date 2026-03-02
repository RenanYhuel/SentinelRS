#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"
GRPC="${GRPC_ADDR:-127.0.0.1:50061}"
WAL_DIR="${TMPDIR:-/tmp}/sentinel-e2e-restart-wal"

rm -rf "$WAL_DIR"
mkdir -p "$WAL_DIR"

echo "  Step 1: Bootstrap agent"
TOKEN_RESP=$(sentinel_cli --server "$REST" agents generate-install \
  --agent-name "e2e-restart" --json 2>&1)
TOKEN=$(echo "$TOKEN_RESP" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
[ -n "$TOKEN" ] || { echo "no token"; exit 1; }

SENTINEL_BOOTSTRAP_TOKEN="$TOKEN" \
SENTINEL_SERVER_URL="$GRPC" \
SENTINEL_WAL_DIR="$WAL_DIR" \
  sentinel_agent --bootstrap &
AGENT_PID=$!
sleep 3

AGENTS=$(curl -sf "$REST/v1/agents")
echo "$AGENTS" | grep -q "e2e-restart" || { echo "agent not registered"; kill $AGENT_PID 2>/dev/null; exit 1; }

echo "  Step 2: Stop agent"
kill $AGENT_PID 2>/dev/null || true
wait $AGENT_PID 2>/dev/null || true
sleep 1

CLUSTER=$(curl -sf "$REST/v1/cluster/stats")
echo "  Cluster after stop: $CLUSTER"

echo "  Step 3: Restart agent (should reconnect, not re-bootstrap)"
CONFIG_DIR="$WAL_DIR"
SENTINEL_CONFIG_DIR="$CONFIG_DIR" \
SENTINEL_WAL_DIR="$WAL_DIR" \
  sentinel_agent &
AGENT_PID=$!
sleep 3

echo "  Step 4: Verify reconnection"
CLUSTER=$(curl -sf "$REST/v1/cluster/stats")
CONNECTED=$(echo "$CLUSTER" | grep -o '"connected_agents":[0-9]*' | cut -d: -f2)
[ "$CONNECTED" -ge 1 ] || { echo "agent not reconnected"; kill $AGENT_PID 2>/dev/null; exit 1; }

echo "  Step 5: Verify agent identity preserved"
AGENTS=$(curl -sf "$REST/v1/agents")
COUNT=$(echo "$AGENTS" | grep -o "e2e-restart" | wc -l)
[ "$COUNT" -eq 1 ] || { echo "expected 1 agent entry, got $COUNT (re-bootstrapped?)"; kill $AGENT_PID 2>/dev/null; exit 1; }

kill $AGENT_PID 2>/dev/null || true
wait $AGENT_PID 2>/dev/null || true
rm -rf "$WAL_DIR"

echo "  Scenario complete"
