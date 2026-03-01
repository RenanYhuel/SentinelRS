#!/usr/bin/env bash
set -euo pipefail

MAX_ATTEMPTS="${RETRY_MAX:-3}"
DELAY="${RETRY_DELAY:-2}"
COMMAND="$*"

if [ -z "$COMMAND" ]; then
  echo "Usage: retry.sh <command> [args...]"
  echo "  RETRY_MAX=3   max attempts (env)"
  echo "  RETRY_DELAY=2 seconds between retries (env)"
  exit 1
fi

for attempt in $(seq 1 "$MAX_ATTEMPTS"); do
  echo "[retry] attempt $attempt/$MAX_ATTEMPTS"
  if eval "$COMMAND"; then
    echo "[retry] success on attempt $attempt"
    exit 0
  fi
  if [ "$attempt" -lt "$MAX_ATTEMPTS" ]; then
    echo "[retry] waiting ${DELAY}s before next attempt..."
    sleep "$DELAY"
    DELAY=$((DELAY * 2))
  fi
done

echo "[retry] all $MAX_ATTEMPTS attempts failed"
exit 1
