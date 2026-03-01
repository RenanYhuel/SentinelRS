#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"
GRPC="${GRPC_ADDR:-127.0.0.1:50061}"

echo "  Step 1: Register agent"
REGISTER_RESP=$(grpcurl -plaintext -d '{"hw_id":"e2e-hw-push","agent_version":"0.1.0"}' \
  "$GRPC" sentinel.common.AgentService/Register 2>&1)

AGENT_ID=$(echo "$REGISTER_RESP" | grep -o '"agentId":"[^"]*"' | cut -d'"' -f4)
SECRET=$(echo "$REGISTER_RESP" | grep -o '"secret":"[^"]*"' | cut -d'"' -f4)
[ -n "$AGENT_ID" ] || { echo "no agent_id"; exit 1; }
echo "  Agent: $AGENT_ID"

echo "  Step 2: Push metrics via gRPC"
PUSH_RESP=$(grpcurl -plaintext \
  -H "x-agent-id: $AGENT_ID" \
  -H "x-signature: placeholder" \
  -H "x-key-id: default" \
  -d "{
    \"agent_id\": \"$AGENT_ID\",
    \"batch_id\": \"e2e-batch-001\",
    \"seq_start\": 0,
    \"seq_end\": 1,
    \"created_at_ms\": 1700000000000,
    \"metrics\": [{
      \"name\": \"cpu.usage\",
      \"rtype\": 1,
      \"value\": {\"valueDouble\": 42.0},
      \"timestamp_ms\": 1700000000000
    }]
  }" \
  "$GRPC" sentinel.common.AgentService/PushMetrics 2>&1) || true

echo "  Push response: $PUSH_RESP"

echo "  Step 3: Send heartbeat"
HB_RESP=$(grpcurl -plaintext \
  -d "{\"agent_id\": \"$AGENT_ID\", \"ts_ms\": 1700000000000}" \
  "$GRPC" sentinel.common.AgentService/SendHeartbeat 2>&1) || true

echo "  Heartbeat response: $HB_RESP"

echo "  Step 4: Verify agent still listed"
AGENTS=$(curl -sf "$REST/v1/agents")
echo "$AGENTS" | grep -q "$AGENT_ID" || { echo "agent missing after push"; exit 1; }

echo "  Scenario complete"
