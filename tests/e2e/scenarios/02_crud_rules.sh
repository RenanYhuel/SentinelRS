#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"

echo "  Step 1: Create alert rule"
RULE_RESP=$(curl -sf -X POST "$REST/v1/rules" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "e2e-high-cpu",
    "metric_name": "cpu.usage",
    "condition": "GreaterThan",
    "threshold": 80.0,
    "severity": "critical"
  }')

RULE_ID=$(echo "$RULE_RESP" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
[ -n "$RULE_ID" ] || { echo "no rule_id in response"; exit 1; }
echo "  Created rule: $RULE_ID"

echo "  Step 2: List rules"
RULES=$(curl -sf "$REST/v1/rules")
echo "$RULES" | grep -q "$RULE_ID" || { echo "rule not found in list"; exit 1; }

echo "  Step 3: Update rule"
UPDATE_RESP=$(curl -sf -X PUT "$REST/v1/rules/$RULE_ID" \
  -H "Content-Type: application/json" \
  -d '{"threshold": 95.0, "name": "e2e-high-cpu-updated"}')

echo "$UPDATE_RESP" | grep -q "95" || { echo "update did not apply"; exit 1; }

echo "  Step 4: Delete rule"
DEL_STATUS=$(curl -sf -o /dev/null -w "%{http_code}" -X DELETE "$REST/v1/rules/$RULE_ID")
[ "$DEL_STATUS" = "200" ] || [ "$DEL_STATUS" = "204" ] || { echo "delete returned $DEL_STATUS"; exit 1; }

echo "  Step 5: Verify deletion"
GET_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$REST/v1/rules/$RULE_ID")
[ "$GET_STATUS" = "404" ] || { echo "deleted rule still accessible ($GET_STATUS)"; exit 1; }

echo "  Scenario complete"
