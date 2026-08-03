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
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod config >/dev/null
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod up -d --build

echo "Local health checks:"
curl -sf http://127.0.0.1:${API_HOST_PORT:-8080}/health && echo " - /health OK"
curl -sf http://127.0.0.1:${API_HOST_PORT:-8080}/health/deps && echo " - /health/deps OK"