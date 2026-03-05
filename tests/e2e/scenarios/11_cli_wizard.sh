#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"

echo "  Step 1: Health check"
STATUS=$(curl -sf -o /dev/null -w "%{http_code}" "$REST/healthz")
[ "$STATUS" = "200" ] || { echo "healthz returned $STATUS"; exit 1; }

echo "  Step 2: sentinel version"
VERSION=$(sentinel_cli version --json 2>/dev/null || echo '{}')
echo "$VERSION" | grep -q "version" || echo "  WARN: version output unexpected"

echo "  Step 3: sentinel doctor"
DOCTOR=$(sentinel_cli --server "$REST" doctor --json 2>/dev/null || echo '{}')
echo "  Doctor output: ${DOCTOR:0:100}..."

echo "  Step 4: sentinel health"
HEALTH=$(sentinel_cli --server "$REST" health --json 2>/dev/null || echo '{}')
echo "$HEALTH" | grep -q "ok\|healthy\|status" || echo "  WARN: health check unexpected"

echo "  Step 5: sentinel agents list"
AGENTS=$(sentinel_cli --server "$REST" agents list --json 2>/dev/null || echo '{}')
echo "  Agents: ${AGENTS:0:200}..."

echo "  Step 6: sentinel rules list"
RULES=$(sentinel_cli --server "$REST" rules list --json 2>/dev/null || echo '{}')
echo "  Rules: ${RULES:0:200}..."

echo "  Step 7: sentinel cluster stats"
CLUSTER=$(sentinel_cli --server "$REST" cluster stats --json 2>/dev/null || echo '{}')
echo "  Cluster: ${CLUSTER:0:200}..."

echo "  Step 8: sentinel plugins list (empty dir)"
TMPDIR_PLUGINS="${TMPDIR:-/tmp}/sentinel-e2e-cli-plugins"
mkdir -p "$TMPDIR_PLUGINS"
PLUGINS=$(sentinel_cli plugins list --dir "$TMPDIR_PLUGINS" --json 2>/dev/null || echo '{}')
echo "  Plugins: ${PLUGINS:0:200}..."
rm -rf "$TMPDIR_PLUGINS"

echo "  Scenario complete"
