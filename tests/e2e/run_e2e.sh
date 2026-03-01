#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/docker-compose.e2e.yml"

export NATS_URL="nats://127.0.0.1:14222"
export DATABASE_URL="postgres://postgres:postgres@127.0.0.1:15432/sentinel_e2e"
export GRPC_ADDR="127.0.0.1:50061"
export REST_ADDR="127.0.0.1:8090"

cleanup() {
  echo "--- Tearing down containers ---"
  docker compose -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || true
  [ -n "${SERVER_PID:-}" ] && kill "$SERVER_PID" 2>/dev/null || true
  [ -n "${WORKER_PID:-}" ] && kill "$WORKER_PID" 2>/dev/null || true
}
trap cleanup EXIT

echo "--- Starting E2E infrastructure ---"
docker compose -f "$COMPOSE_FILE" up -d --wait

echo "--- Waiting for NATS ---"
for i in $(seq 1 30); do
  curl -sf "http://127.0.0.1:18222/healthz" >/dev/null 2>&1 && break
  sleep 1
done

echo "--- Waiting for TimescaleDB ---"
for i in $(seq 1 30); do
  PGPASSWORD=postgres psql -h 127.0.0.1 -p 15432 -U postgres -d sentinel_e2e -c "SELECT 1" >/dev/null 2>&1 && break
  sleep 1
done

echo "--- Building binaries ---"
cargo build --release -p sentinel_server -p sentinel_workers 2>/dev/null

echo "--- Starting server ---"
SENTINEL_GRPC_ADDR="$GRPC_ADDR" \
SENTINEL_REST_ADDR="$REST_ADDR" \
SENTINEL_NATS_URL="$NATS_URL" \
  cargo run --release -p sentinel_server &
SERVER_PID=$!
sleep 2

echo "--- Starting worker ---"
SENTINEL_NATS_URL="$NATS_URL" \
SENTINEL_DATABASE_URL="$DATABASE_URL" \
  cargo run --release -p sentinel_workers &
WORKER_PID=$!
sleep 2

echo "--- Running E2E scenarios ---"
FAILURES=0

run_scenario() {
  local name="$1"
  local script="$2"
  echo ""
  echo "=== Scenario: $name ==="
  if bash "$script"; then
    echo "  -> PASS"
  else
    echo "  -> FAIL"
    FAILURES=$((FAILURES + 1))
  fi
}

for scenario in "$SCRIPT_DIR"/scenarios/*.sh; do
  [ -f "$scenario" ] || continue
  name="$(basename "$scenario" .sh)"
  run_scenario "$name" "$scenario"
done

echo ""
echo "--- E2E Summary ---"
if [ "$FAILURES" -eq 0 ]; then
  echo "All scenarios passed."
  exit 0
else
  echo "$FAILURES scenario(s) failed."
  exit 1
fi
