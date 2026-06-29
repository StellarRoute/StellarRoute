#!/bin/bash
# Wait for local Docker Compose services required by the API and indexer.

set -euo pipefail

TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-60}"
SLEEP_SECONDS="${SLEEP_SECONDS:-2}"

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

print_status() {
    echo ""
    echo "Current service status:"
    run_compose ps postgres redis 2>/dev/null || true
}

deadline=$((SECONDS + TIMEOUT_SECONDS))
echo "Waiting up to ${TIMEOUT_SECONDS}s for Postgres and Redis to become healthy..."

while (( SECONDS < deadline )); do
    postgres_ok=false
    redis_ok=false

    if postgres_ready; then
        postgres_ok=true
    fi

    if redis_ready; then
        redis_ok=true
    fi

    if [[ "${postgres_ok}" == "true" && "${redis_ok}" == "true" ]]; then
        echo "[OK] Postgres and Redis are ready."
        exit 0
    fi

    echo "Still waiting: postgres=${postgres_ok}, redis=${redis_ok}"
    sleep "${SLEEP_SECONDS}"
done

echo "[ERROR] Timed out after ${TIMEOUT_SECONDS}s waiting for Postgres and Redis." >&2
print_status >&2
cat >&2 <<'EOF'

Next steps:
  1. Start or restart dependencies: docker-compose up -d
  2. Inspect health checks: docker-compose ps
  3. Review recent logs:
       docker-compose logs --tail=50 postgres
       docker-compose logs --tail=50 redis

You can extend the wait window with:
  TIMEOUT_SECONDS=120 ./scripts/wait-for-services.sh
EOF
exit 1
