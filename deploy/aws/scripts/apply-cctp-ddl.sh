#!/usr/bin/env bash
# Bootstrap any pre-existing tables into `_api_schema_migrations` and apply all
# pending `crates/api/migrations/*.sql` files via the API migration runner.
# Idempotent. Run from repo root on the staging host.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "${ROOT}"

ENV_FILE="${ROOT}/.env.prod"
if [[ ! -f "${ENV_FILE}" ]]; then
  echo "Missing ${ENV_FILE}." >&2
  exit 1
fi

POSTGRES_USER="$(grep -E '^POSTGRES_USER=' "${ENV_FILE}" | head -1 | cut -d= -f2-)"
POSTGRES_DB="$(grep -E '^POSTGRES_DB=' "${ENV_FILE}" | head -1 | cut -d= -f2-)"
if [[ -z "${POSTGRES_USER}" || -z "${POSTGRES_DB}" ]]; then
  echo "POSTGRES_USER / POSTGRES_DB missing from ${ENV_FILE}" >&2
  exit 1
fi

COMPOSE=(docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod)

echo "Ensuring postgres service is running before API migrations..."
"${COMPOSE[@]}" up -d postgres

echo "Running API migration runner (migrate --bootstrap)..."
"${COMPOSE[@]}" build api
"${COMPOSE[@]}" run --rm --no-deps api migrate --bootstrap
echo " - API migrations up to date"
