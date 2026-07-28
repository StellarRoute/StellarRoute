#!/bin/bash
# Wait for local Docker Compose services required by the API and indexer.
#
# By default waits for Postgres and Redis (deps-only stack).
# Pass --api to also wait for the API health endpoint.
#
# Usage:
#   ./scripts/wait-for-services.sh              # deps only
#   ./scripts/wait-for-services.sh --api        # deps + API
#   TIMEOUT_SECONDS=120 ./scripts/wait-for-services.sh --api

set -euo pipefail

TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-60}"
SLEEP_SECONDS="${SLEEP_SECONDS:-2}"
WAIT_FOR_API=false
API_PORT="${API_PORT:-3000}"

for arg in "$@"; do
    case "$arg" in
        --api) WAIT_FOR_API=true ;;
    esac
done

if command -v docker-compose >/dev/null 2>&1; then
    COMPOSE=(docker-compose)
elif docker compose version >/dev/null 2>&1; then
    COMPOSE=(docker compose)
else
    echo "[ERROR] Docker Compose is not available." >&2
    echo "Install Docker Compose, then run: docker-compose up -d" >&2
    exit 1
fi

run_compose() {
    "${COMPOSE[@]}" "$@"
}

postgres_ready() {
    run_compose exec -T postgres pg_isready -U stellarroute -d stellarroute >/dev/null 2>&1
}

redis_ready() {
    local response
    response="$(run_compose exec -T redis redis-cli ping 2>/dev/null | tr -d '\r' || true)"
    [[ "${response}" == "PONG" ]]
}

api_ready() {
    curl -sf "http://127.0.0.1:${API_PORT}/health" >/dev/null 2>&1
}

print_status() {
    echo ""
    echo "Current service status:"
    run_compose ps 2>/dev/null || true
}

deadline=$((SECONDS + TIMEOUT_SECONDS))

if [[ "${WAIT_FOR_API}" == "true" ]]; then
    echo "Waiting up to ${TIMEOUT_SECONDS}s for Postgres, Redis, and API (port ${API_PORT})..."
else
    echo "Waiting up to ${TIMEOUT_SECONDS}s for Postgres and Redis to become healthy..."
fi

while (( SECONDS < deadline )); do
    postgres_ok=false
    redis_ok=false
    api_ok=false

    postgres_ready && postgres_ok=true
    redis_ready    && redis_ok=true

    if [[ "${WAIT_FOR_API}" == "true" ]]; then
        api_ready && api_ok=true

        if [[ "${postgres_ok}" == "true" && "${redis_ok}" == "true" && "${api_ok}" == "true" ]]; then
            echo "[OK] Postgres, Redis, and API are ready."
            exit 0
        fi
        echo "Still waiting: postgres=${postgres_ok}, redis=${redis_ok}, api=${api_ok}"
    else
        if [[ "${postgres_ok}" == "true" && "${redis_ok}" == "true" ]]; then
            echo "[OK] Postgres and Redis are ready."
            exit 0
        fi
        echo "Still waiting: postgres=${postgres_ok}, redis=${redis_ok}"
    fi

    sleep "${SLEEP_SECONDS}"
done

echo "[ERROR] Timed out after ${TIMEOUT_SECONDS}s." >&2
print_status >&2
cat >&2 <<'EOF'

Next steps:
  1. Start or restart services:
       # deps only:
       docker-compose up -d
       # full stack (deps + API):
       docker compose -f docker-compose.yml -f docker-compose.app.yml up -d
       # full stack with indexer:
       docker compose -f docker-compose.yml -f docker-compose.app.yml --profile indexer up -d
  2. Inspect health checks: docker-compose ps
  3. Review recent logs:
       docker-compose logs --tail=50 postgres
       docker-compose logs --tail=50 redis
       docker compose -f docker-compose.yml -f docker-compose.app.yml logs --tail=50 api

You can extend the wait window with:
  TIMEOUT_SECONDS=120 ./scripts/wait-for-services.sh [--api]
EOF
exit 1
