#!/usr/bin/env bash
# One-command smoke test for the EC2 staging deployment.
# Run from the EC2 host after the stack and Caddy are up:
#   bash deploy/aws/scripts/ec2-postdeploy-smoke.sh https://api-staging.example.com
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
PUBLIC_URL="${1:-${STAGING_API_BASE_URL:-}}"
LOCAL_PORT="${API_HOST_PORT:-8080}"

if [[ -z "${PUBLIC_URL}" ]]; then
  echo "Usage: bash deploy/aws/scripts/ec2-postdeploy-smoke.sh <public-url>" >&2
  echo "Example: bash deploy/aws/scripts/ec2-postdeploy-smoke.sh https://api-staging.example.com" >&2
  exit 1
fi

PUBLIC_URL="${PUBLIC_URL%/}"
HOSTNAME_ONLY="${PUBLIC_URL#https://}"
HOSTNAME_ONLY="${HOSTNAME_ONLY#http://}"
HOSTNAME_ONLY="${HOSTNAME_ONLY%%/*}"

echo "EC2 staging smoke for ${PUBLIC_URL}"

echo "[1/6] Docker Compose service status"
cd "${ROOT}"
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod ps

echo "[2/6] Local API health"
curl -sf "http://127.0.0.1:${LOCAL_PORT}/health" >/dev/null
curl -sf "http://127.0.0.1:${LOCAL_PORT}/health/deps" >/dev/null
echo "Local health checks passed on 127.0.0.1:${LOCAL_PORT}"

echo "[3/6] Caddy service state"
sudo systemctl is-active --quiet caddy
echo "Caddy is active"

echo "[4/6] DNS resolution"
getent hosts "${HOSTNAME_ONLY}" || host "${HOSTNAME_ONLY}" || nslookup "${HOSTNAME_ONLY}"

echo "[5/6] Public HTTPS health"
curl -sf "${PUBLIC_URL}/health" >/dev/null
curl -sf "${PUBLIC_URL}/health/deps" >/dev/null
echo "Public HTTPS health checks passed"

echo "[6/6] Full API smoke"
STAGING_API_BASE_URL="${PUBLIC_URL}" "${ROOT}/scripts/staging-smoke.sh"

echo "EC2 post-deploy smoke passed."