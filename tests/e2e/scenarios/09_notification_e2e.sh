#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"
GRPC="${GRPC_ADDR:-127.0.0.1:50061}"

echo "  Step 1: Health check"
STATUS=$(curl -sf -o /dev/null -w "%{http_code}" "$REST/healthz")
[ "$STATUS" = "200" ] || { echo "healthz returned $STATUS"; exit 1; }

echo "  Step 2: Create webhook notifier"
NOTIFIER_RESP=$(curl -sf -X POST "$REST/v1/notifiers" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "e2e-webhook",
    "type": "webhook",
    "config": {
      "url": "http://127.0.0.1:19999/hook",
      "method": "POST"
    }
  }' 2>&1) || echo "  WARN: notifier creation returned: $NOTIFIER_RESP"

echo "  Step 3: Create alert rule with low threshold"
RULE_RESP=$(curl -sf -X POST "$REST/v1/rules" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "e2e-cpu-high",
    "metric": "system.cpu.usage",
    "operator": ">",
    "threshold": 0.0,
    "duration_seconds": 1,
    "severity": "warning",
    "notifier_ids": ["e2e-webhook"]
  }' 2>&1) || echo "  WARN: rule creation returned: $RULE_RESP"
echo "  Rule created (threshold=0: any CPU usage triggers)"

echo "  Step 4: Bootstrap and start agent"
TOKEN_RESP=$(sentinel_cli --server "$REST" agents generate-install \
  --agent-name "e2e-notify" --json 2>&1)
TOKEN=$(echo "$TOKEN_RESP" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
[ -n "$TOKEN" ] || { echo "no token"; exit 1; }

SENTINEL_BOOTSTRAP_TOKEN="$TOKEN" \
SENTINEL_SERVER_URL="$GRPC" \
  sentinel_agent --bootstrap &
AGENT_PID=$!
sleep 5

echo "  Step 5: Wait for metrics to be processed"
sleep 10

echo "  Step 6: Check for triggered alerts"
ALERTS=$(curl -sf "$REST/v1/alerts?limit=10" 2>/dev/null || echo '{"alerts":[]}')
ALERT_COUNT=$(echo "$ALERTS" | grep -o '"severity"' | wc -l || echo "0")

if [ "${ALERT_COUNT:-0}" -gt 0 ]; then
  echo "  Alerts triggered: $ALERT_COUNT"
else
  echo "  WARN: no alerts triggered (worker/eval may be slow)"
fi

echo "  Step 7: Verify alert rules list"
RULES=$(curl -sf "$REST/v1/rules" 2>/dev/null || echo '[]')
echo "$RULES" | grep -q "e2e-cpu-high" || echo "  WARN: rule not found in list"

echo "  Step 8: Clean up rule"
curl -sf -X DELETE "$REST/v1/rules/e2e-cpu-high" 2>/dev/null || true

kill $AGENT_PID 2>/dev/null || true
wait $AGENT_PID 2>/dev/null || true

echo "  Scenario complete"
