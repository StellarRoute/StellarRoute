#!/usr/bin/env bash
# Restore a compressed Postgres backup into the EC2 single-host staging stack.
# Example:
#   sudo bash deploy/aws/scripts/postgres-restore.sh /var/backups/stellarroute-postgres/stellarroute-20260803T120000Z.sql.gz --yes
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
ENV_FILE="${ROOT}/.env.prod"
CONTAINER_NAME="${POSTGRES_CONTAINER_NAME:-stellarroute-postgres}"

BACKUP_FILE="${1:-}"
CONFIRM="${2:-}"

if [[ -z "${BACKUP_FILE}" || "${CONFIRM}" != "--yes" ]]; then
  echo "Usage: sudo bash deploy/aws/scripts/postgres-restore.sh <backup.sql.gz> --yes" >&2
  exit 1
fi

if [[ ! -f "${BACKUP_FILE}" ]]; then
  echo "Backup file not found: ${BACKUP_FILE}" >&2
  exit 1
fi

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "Missing ${ENV_FILE}." >&2
  exit 1
fi

set -a
source "${ENV_FILE}"
set +a

: "${POSTGRES_USER:?POSTGRES_USER is required in .env.prod}"
: "${POSTGRES_DB:?POSTGRES_DB is required in .env.prod}"

docker ps --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"

echo "Stopping API and indexer to avoid writes during restore..."
cd "${ROOT}"
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod stop api indexer

restore_exit=0
if ! docker exec -e PGPASSWORD="${POSTGRES_PASSWORD:-}" "${CONTAINER_NAME}" \
  psql -U "${POSTGRES_USER}" -d postgres -v ON_ERROR_STOP=1 \
  -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '${POSTGRES_DB}' AND pid <> pg_backend_pid();" \
  -c "DROP DATABASE IF EXISTS \"${POSTGRES_DB}\";" \
  -c "CREATE DATABASE \"${POSTGRES_DB}\";"; then
  restore_exit=1
fi

if [[ "${restore_exit}" -eq 0 ]]; then
  if ! gzip -dc "${BACKUP_FILE}" | docker exec -i -e PGPASSWORD="${POSTGRES_PASSWORD:-}" "${CONTAINER_NAME}" \
    psql -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -v ON_ERROR_STOP=1; then
    restore_exit=1
  fi
fi

echo "Restarting API and indexer..."
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod up -d api indexer

if [[ "${restore_exit}" -ne 0 ]]; then
  echo "Restore failed." >&2
  exit 1
fi

echo "Restore completed from ${BACKUP_FILE}"