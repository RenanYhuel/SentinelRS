#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

VERSION="${1:-latest}"
REGISTRY="${DOCKER_REGISTRY:-sentinelrs}"

COMPONENTS=("server" "worker" "agent" "cli")

echo "==> Building SentinelRS Docker images (version: ${VERSION})"
echo ""

for component in "${COMPONENTS[@]}"; do
    TAG="${REGISTRY}/${component}:${VERSION}"
    DOCKERFILE="${ROOT_DIR}/docker/Dockerfile.${component}"

    echo "--- Building ${TAG}"
    docker build \
        -f "${DOCKERFILE}" \
        -t "${TAG}" \
        -t "${REGISTRY}/${component}:latest" \
        --label "org.opencontainers.image.version=${VERSION}" \
        --label "org.opencontainers.image.created=$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        "${ROOT_DIR}"
    echo "--- Done: ${TAG}"
    echo ""
done

echo "==> All images built:"
for component in "${COMPONENTS[@]}"; do
    echo "  ${REGISTRY}/${component}:${VERSION}"
done
