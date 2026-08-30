#!/usr/bin/env bash
# Create a compressed Postgres backup from the EC2 single-host staging stack.
# Example:
#   sudo bash deploy/aws/scripts/postgres-backup.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
ENV_FILE="${ROOT}/.env.prod"
BACKUP_DIR="${BACKUP_DIR:-/var/backups/stellarroute-postgres}"
CONTAINER_NAME="${POSTGRES_CONTAINER_NAME:-stellarroute-postgres}"
RETENTION_DAYS="${RETENTION_DAYS:-7}"

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "Missing ${ENV_FILE}." >&2
  exit 1
fi

set -a
source "${ENV_FILE}"
set +a

: "${POSTGRES_USER:?POSTGRES_USER is required in .env.prod}"
: "${POSTGRES_DB:?POSTGRES_DB is required in .env.prod}"

mkdir -p "${BACKUP_DIR}"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
outfile="${BACKUP_DIR}/${POSTGRES_DB}-${timestamp}.sql.gz"

docker ps --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"

docker exec -e PGPASSWORD="${POSTGRES_PASSWORD:-}" "${CONTAINER_NAME}" \
  pg_dump -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" \
  | gzip -9 >"${outfile}"

find "${BACKUP_DIR}" -type f -name '*.sql.gz' -mtime +"${RETENTION_DAYS}" -delete

echo "Backup created: ${outfile}"