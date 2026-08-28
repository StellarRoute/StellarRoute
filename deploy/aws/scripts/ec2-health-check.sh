#!/usr/bin/env bash
# Basic health check for the single-host EC2 staging deployment.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
DISK_WARN_PERCENT="${DISK_WARN_PERCENT:-85}"
LOCAL_PORT="${API_HOST_PORT:-8080}"

cd "${ROOT}"

docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod ps --status running >/dev/null
systemctl is-active --quiet docker
systemctl is-active --quiet caddy

root_use="$(df -P / | awk 'NR==2 {gsub(/%/, "", $5); print $5}')"
if (( root_use >= DISK_WARN_PERCENT )); then
  echo "Disk usage critical: ${root_use}% used on /" >&2
  exit 1
fi

curl -sf "http://127.0.0.1:${LOCAL_PORT}/health" >/dev/null
curl -sf "http://127.0.0.1:${LOCAL_PORT}/health/deps" >/dev/null

echo "EC2 health check OK: disk=${root_use}% caddy=active docker=active"