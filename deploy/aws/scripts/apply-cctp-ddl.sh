#!/usr/bin/env bash
# Apply every crates/api/migrations/*cctp*.sql file in sorted order.
# Idempotent. Run from repo root on the staging host (Compose stack up).
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

shopt -s nullglob
migrations=("${ROOT}"/crates/api/migrations/*cctp*.sql)
if [[ ${#migrations[@]} -eq 0 ]]; then
  echo "No CCTP SQL migrations found under crates/api/migrations" >&2
  exit 1
fi

IFS=$'\n' sorted=($(printf '%s\n' "${migrations[@]}" | sort))
unset IFS

echo "Applying ${#sorted[@]} CCTP DDL files..."
COMPOSE=(docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod)
for mig in "${sorted[@]}"; do
  "${COMPOSE[@]}" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" \
    < "${mig}"
  echo " - $(basename "${mig}") ready"
done
