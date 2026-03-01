#!/usr/bin/env bash
set -euo pipefail

NATS_URL="${NATS_URL:-nats://localhost:4222}"
STREAM_NAME="SENTINEL_METRICS"
SUBJECT="sentinel.metrics.>"
CONSUMER="sentinel-workers"

echo "==> Creating JetStream stream: ${STREAM_NAME}"
nats stream add "${STREAM_NAME}" \
  --subjects="${SUBJECT}" \
  --retention=limits \
  --storage=file \
  --max-bytes=1073741824 \
  --max-age=7d \
  --max-msg-size=1048576 \
  --discard=old \
  --replicas=1 \
  --server="${NATS_URL}" \
  --defaults 2>/dev/null || \
nats stream update "${STREAM_NAME}" \
  --server="${NATS_URL}" \
  --force 2>/dev/null || true

echo "==> Creating durable consumer: ${CONSUMER}"
nats consumer add "${STREAM_NAME}" "${CONSUMER}" \
  --pull \
  --deliver=all \
  --ack=explicit \
  --max-deliver=5 \
  --filter="" \
  --replay=instant \
  --server="${NATS_URL}" \
  --defaults 2>/dev/null || true

echo "==> Done. Stream info:"
nats stream info "${STREAM_NAME}" --server="${NATS_URL}"
