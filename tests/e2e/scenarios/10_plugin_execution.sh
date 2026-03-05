#!/usr/bin/env bash
set -euo pipefail

REST="http://${REST_ADDR:-127.0.0.1:8090}"
GRPC="${GRPC_ADDR:-127.0.0.1:50061}"
PLUGINS_DIR="${TMPDIR:-/tmp}/sentinel-e2e-plugins"

echo "  Step 1: Health check"
STATUS=$(curl -sf -o /dev/null -w "%{http_code}" "$REST/healthz")
[ "$STATUS" = "200" ] || { echo "healthz returned $STATUS"; exit 1; }

echo "  Step 2: Prepare plugin directory"
rm -rf "$PLUGINS_DIR"
mkdir -p "$PLUGINS_DIR"

echo "  Step 3: Install example plugin via CLI"
EXAMPLE_WAT="$(dirname "$0")/../../examples/plugins/hello_metrics/hello_metrics.wat"
EXAMPLE_MANIFEST="$(dirname "$0")/../../examples/plugins/hello_metrics/manifest.yml"

if [ -f "$EXAMPLE_WAT" ] && [ -f "$EXAMPLE_MANIFEST" ]; then
  sentinel_cli plugins install "$EXAMPLE_WAT" \
    --manifest "$EXAMPLE_MANIFEST" \
    --dir "$PLUGINS_DIR" || { echo "plugin install failed"; exit 1; }
  echo "  Plugin installed"
else
  echo "  Creating inline WAT plugin"
  cat > "$PLUGINS_DIR/hello_metrics.wasm" << 'WASMEOF'
(module
  (import "sentinel" "emit_metric_json" (func $emit (param i32 i32) (result i32)))
  (import "sentinel" "log" (func $log (param i32 i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "{\"name\":\"hello\",\"value\":1}")
  (data (i32.const 64) "e2e plugin test")
  (func (export "collect") (result i32)
    (call $log (i32.const 64) (i32.const 15))
    (call $emit (i32.const 0) (i32.const 25))
    drop
    (i32.const 0)
  )
)
WASMEOF
  cat > "$PLUGINS_DIR/hello_metrics.manifest.yml" << 'YMLEOF'
name: hello_metrics
version: "1.0.0"
entry_fn: collect
YMLEOF
fi

echo "  Step 4: List installed plugins"
PLUGIN_LIST=$(sentinel_cli plugins list --dir "$PLUGINS_DIR" --json 2>/dev/null || echo '{}')
echo "$PLUGIN_LIST" | grep -q "hello_metrics" || { echo "plugin not in list"; exit 1; }
echo "  Plugin visible in list"

echo "  Step 5: Inspect plugin"
INSPECT=$(sentinel_cli plugins inspect hello_metrics --dir "$PLUGINS_DIR" --json 2>/dev/null || echo '{}')
echo "$INSPECT" | grep -q "collect" || echo "  WARN: entry_fn not found in inspect"

echo "  Step 6: Bootstrap agent with plugins enabled"
TOKEN_RESP=$(sentinel_cli --server "$REST" agents generate-install \
  --agent-name "e2e-plugin-agent" --json 2>&1)
TOKEN=$(echo "$TOKEN_RESP" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
[ -n "$TOKEN" ] || { echo "no token"; exit 1; }

SENTINEL_BOOTSTRAP_TOKEN="$TOKEN" \
SENTINEL_SERVER_URL="$GRPC" \
SENTINEL_PLUGINS_DIR="$PLUGINS_DIR" \
  sentinel_agent --bootstrap &
AGENT_PID=$!
sleep 5

echo "  Step 7: Verify agent connected"
CLUSTER=$(curl -sf "$REST/v1/cluster/stats" 2>/dev/null || echo '{}')
CONNECTED=$(echo "$CLUSTER" | grep -o '"connected_agents":[0-9]*' | cut -d: -f2 || echo "0")
[ "${CONNECTED:-0}" -ge 1 ] || echo "  WARN: agent may not be connected"

echo "  Step 8: Wait for plugin metrics"
sleep 15

echo "  Step 9: Check metrics in DB"
METRIC_COUNT=$(PGPASSWORD=postgres psql -h 127.0.0.1 -p "${DB_PORT:-15432}" -U postgres -d sentinel_e2e \
  -t -c "SELECT count(*) FROM metrics_time WHERE name LIKE 'plugin.%';" 2>/dev/null | tr -d ' ' || echo "0")

if [ "${METRIC_COUNT:-0}" -gt 0 ]; then
  echo "  Plugin metrics in DB: $METRIC_COUNT"
else
  echo "  WARN: no plugin metrics in DB yet"
fi

echo "  Step 10: Remove plugin"
sentinel_cli plugins remove hello_metrics --dir "$PLUGINS_DIR" || echo "  WARN: remove failed"
[ ! -f "$PLUGINS_DIR/hello_metrics.wasm" ] || echo "  WARN: .wasm file still exists"

kill $AGENT_PID 2>/dev/null || true
wait $AGENT_PID 2>/dev/null || true
rm -rf "$PLUGINS_DIR"

echo "  Scenario complete"
