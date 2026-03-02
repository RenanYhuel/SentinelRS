#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"
GRPC="${GRPC_ADDR:-127.0.0.1:50061}"
WAL_DIR="${TMPDIR:-/tmp}/sentinel-e2e-partition-wal"

rm -rf "$WAL_DIR"
mkdir -p "$WAL_DIR"

echo "  Step 1: Bootstrap agent"
TOKEN_RESP=$(sentinel_cli --server "$REST" agents generate-install \
  --agent-name "e2e-partition" --json 2>&1)
TOKEN=$(echo "$TOKEN_RESP" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
[ -n "$TOKEN" ] || { echo "no token"; exit 1; }

SENTINEL_BOOTSTRAP_TOKEN="$TOKEN" \
SENTINEL_SERVER_URL="$GRPC" \
SENTINEL_WAL_DIR="$WAL_DIR" \
  sentinel_agent --bootstrap &
AGENT_PID=$!
sleep 3

echo "  Step 2: Verify connected"
CLUSTER=$(curl -sf "$REST/v1/cluster/stats")
echo "  Cluster: $CLUSTER"

echo "  Step 3: Simulate network partition (block agent → server traffic)"
if command -v iptables &>/dev/null; then
  GRPC_PORT=$(echo "$GRPC" | cut -d: -f2)
  iptables -A OUTPUT -p tcp --dport "$GRPC_PORT" -j DROP 2>/dev/null || true
  IPTABLES_USED=true
else
  echo "  iptables not available, using kill -STOP to pause agent"
  kill -STOP $AGENT_PID 2>/dev/null || true
  IPTABLES_USED=false
fi

echo "  Step 4: Wait for WAL to buffer (agent generates metrics offline)"
sleep 5

echo "  Step 5: Check WAL has pending entries"
WAL_FILES=$(find "$WAL_DIR" -name "*.wal" -o -name "*.batch" 2>/dev/null | wc -l)
echo "  WAL files: $WAL_FILES"

echo "  Step 6: Restore connectivity"
if [ "$IPTABLES_USED" = "true" ]; then
  GRPC_PORT=$(echo "$GRPC" | cut -d: -f2)
  iptables -D OUTPUT -p tcp --dport "$GRPC_PORT" -j DROP 2>/dev/null || true
else
  kill -CONT $AGENT_PID 2>/dev/null || true
fi

echo "  Step 7: Wait for WAL flush"
sleep 8

echo "  Step 8: Verify agent reconnected"
CLUSTER=$(curl -sf "$REST/v1/cluster/stats")
CONNECTED=$(echo "$CLUSTER" | grep -o '"connected_agents":[0-9]*' | cut -d: -f2)
echo "  Connected agents after restore: $CONNECTED"

kill $AGENT_PID 2>/dev/null || true
wait $AGENT_PID 2>/dev/null || true
rm -rf "$WAL_DIR"

echo "  Scenario complete"
