#!/usr/bin/env bash
# scripts/oracle/up.sh — bring up the Wave 0 compose stack on this host.
# Prerequisites: Docker, .env.prod filled (see deploy/env.prod.example).
# Usage (from repo root):
#   ./scripts/oracle/up.sh
#   ./scripts/oracle/up.sh --build
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${ROOT}"

ENV_FILE="${ENV_FILE:-.env.prod}"
if [[ ! -f "${ENV_FILE}" ]]; then
  echo "Missing ${ENV_FILE}. Copy deploy/env.prod.example and fill secrets." >&2
  exit 1
fi

COMPOSE=(docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file "${ENV_FILE}")

BUILD_FLAG=()
if [[ "${1:-}" == "--build" ]]; then
  BUILD_FLAG=(--build)
fi

"${COMPOSE[@]}" config >/dev/null
"${COMPOSE[@]}" up -d "${BUILD_FLAG[@]}"

echo "Waiting for API /health/deps…"
API_HOST_PORT="$(grep -E '^API_HOST_PORT=' "${ENV_FILE}" | cut -d= -f2- || true)"
API_HOST_PORT="${API_HOST_PORT:-8080}"
for _ in $(seq 1 90); do
  if curl -sf "http://127.0.0.1:${API_HOST_PORT}/health/deps" >/dev/null; then
    echo "API deps healthy on http://127.0.0.1:${API_HOST_PORT}"
    "${COMPOSE[@]}" ps
    echo "Note: GET /health may stay 503 until the indexer reduces lag."
    exit 0
  fi
  sleep 5
done

echo "API did not become healthy in time. Logs:" >&2
"${COMPOSE[@]}" logs --tail=80 api >&2 || true
exit 1
