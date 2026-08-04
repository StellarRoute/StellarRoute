#!/usr/bin/env bash
# Deploy the single-host staging stack on EC2 using Docker Compose.
# Run from the repo root on the EC2 instance:
#   bash deploy/aws/scripts/deploy-ec2-staging.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
ENV_FILE="${ROOT}/.env.prod"

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "Missing ${ENV_FILE}. Copy deploy/env.prod.example and fill required values first." >&2
  exit 1
fi

required_vars=(
  POSTGRES_USER
  POSTGRES_PASSWORD
  POSTGRES_DB
  REDIS_PASSWORD
  SOROBAN_RPC_URL
  STELLAR_HORIZON_URL
  ROUTER_CONTRACT_ADDRESS
  CORS_ALLOWED_ORIGINS
  ADMIN_AUTH_TOKEN
)

for key in "${required_vars[@]}"; do
  if ! grep -Eq "^${key}=.+" "${ENV_FILE}"; then
    echo "${key} is missing or empty in ${ENV_FILE}" >&2
    exit 1
  fi
done

cd "${ROOT}"
GIT_SHA="$(git -c safe.directory=* rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "Deploying git SHA ${GIT_SHA}"

docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod config >/dev/null
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod up -d --build

# API does not migrate on boot. Ensure classic swap prepare/submit tables exist
# even if the indexer profile is slow/offline (idempotent DDL).
echo "Applying swap_prepared_quotes DDL (idempotent)..."
POSTGRES_USER="$(grep -E '^POSTGRES_USER=' "${ENV_FILE}" | head -1 | cut -d= -f2-)"
POSTGRES_DB="$(grep -E '^POSTGRES_DB=' "${ENV_FILE}" | head -1 | cut -d= -f2-)"
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod exec -T postgres \
  psql -v ON_ERROR_STOP=1 -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" \
  < "${ROOT}/crates/indexer/migrations/0015_swap_prepared_quotes.sql"
echo " - swap_prepared_quotes ready"

echo "Local health checks:"
# API may still be booting; do not fail the whole deploy on a racing curl here.
# Post-deploy smoke waits for readiness separately.
curl -sf http://127.0.0.1:${API_HOST_PORT:-8080}/health && echo " - /health OK" || echo " - /health not ready yet"
curl -sf http://127.0.0.1:${API_HOST_PORT:-8080}/health/deps && echo " - /health/deps OK" || echo " - /health/deps not ready yet"