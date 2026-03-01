#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"
GRPC="${GRPC_ADDR:-127.0.0.1:50061}"

echo "  Step 1: Health check"
STATUS=$(curl -sf -o /dev/null -w "%{http_code}" "$REST/healthz")
[ "$STATUS" = "200" ] || { echo "healthz returned $STATUS"; exit 1; }

echo "  Step 2: Ready check"
STATUS=$(curl -sf -o /dev/null -w "%{http_code}" "$REST/ready")
[ "$STATUS" = "200" ] || { echo "ready returned $STATUS"; exit 1; }

echo "  Step 3: Register agent via gRPC"
REGISTER_RESP=$(grpcurl -plaintext -d '{"hw_id":"e2e-hw-001","agent_version":"0.1.0"}' \
  "$GRPC" sentinel.common.AgentService/Register 2>&1) || {
  echo "gRPC register failed: $REGISTER_RESP"
  exit 1
}

AGENT_ID=$(echo "$REGISTER_RESP" | grep -o '"agentId":"[^"]*"' | cut -d'"' -f4)
[ -n "$AGENT_ID" ] || { echo "no agent_id in response"; exit 1; }
echo "  Registered: $AGENT_ID"

echo "  Step 4: List agents via REST"
AGENTS=$(curl -sf "$REST/v1/agents")
echo "$AGENTS" | grep -q "$AGENT_ID" || { echo "agent not found in list"; exit 1; }

echo "  Scenario complete"
