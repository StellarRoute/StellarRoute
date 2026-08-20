#!/usr/bin/env bash
# Enable CCTP on EC2 staging by upserting .env.prod and recreating the API container.
#
# Usage (on the staging host, from repo root):
#   CCTP_SEPOLIA_RPC_URL=https://ethereum-sepolia-rpc.publicnode.com bash deploy/aws/scripts/enable-cctp-staging.sh
#
# Defaults:
#   CCTP_ENABLED=true
#   CCTP_SEPOLIA_RPC_URL / SEPOLIA_RPC_URL required (or pass explicitly)
#   CCTP_ACCESS_TOKEN_HMAC_KEY generated if missing/empty
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "${ROOT}"

ENV_FILE="${ROOT}/.env.prod"
if [[ ! -f "${ENV_FILE}" ]]; then
  echo "Missing ${ENV_FILE}. Copy deploy/env.prod.example and fill required values first." >&2
  exit 1
fi

SEPOLIA_RPC="${CCTP_SEPOLIA_RPC_URL:-${SEPOLIA_RPC_URL:-}}"
if [[ -z "${SEPOLIA_RPC}" ]]; then
  echo "CCTP_SEPOLIA_RPC_URL or SEPOLIA_RPC_URL is required (explicit Sepolia JSON-RPC; no silent default)." >&2
  exit 1
fi

upsert_env() {
  local key="$1"
  local value="$2"
  local tmp
  tmp="$(mktemp)"
  if grep -qE "^${key}=" "${ENV_FILE}"; then
    # Preserve other lines; replace matching key only.
    awk -v k="${key}" -v v="${value}" '
      BEGIN { found=0 }
      index($0, k "=") == 1 { print k "=" v; found=1; next }
      { print }
      END { if (!found) print k "=" v }
    ' "${ENV_FILE}" > "${tmp}"
  else
    cat "${ENV_FILE}" > "${tmp}"
    printf '%s=%s\n' "${key}" "${value}" >> "${tmp}"
  fi
  mv "${tmp}" "${ENV_FILE}"
  chmod 600 "${ENV_FILE}" || true
}

# shellcheck disable=SC1090
set -a
# Load existing values without exporting secrets to logs later.
source "${ENV_FILE}"
set +a

HMAC_KEY="${CCTP_ACCESS_TOKEN_HMAC_KEY:-}"
if [[ -z "${HMAC_KEY}" ]]; then
  HMAC_KEY="$(python3 -c 'import os,base64; print(base64.urlsafe_b64encode(os.urandom(32)).decode().rstrip("="))')"
  echo "Generated new CCTP_ACCESS_TOKEN_HMAC_KEY (not printed)."
else
  echo "Reusing existing CCTP_ACCESS_TOKEN_HMAC_KEY."
fi

upsert_env "CCTP_ENABLED" "true"
upsert_env "CCTP_ACCESS_TOKEN_HMAC_KEY" "${HMAC_KEY}"
upsert_env "CCTP_SEPOLIA_RPC_URL" "${SEPOLIA_RPC}"

if [[ -n "${CCTP_STELLAR_RPC_URL:-}" ]]; then
  upsert_env "CCTP_STELLAR_RPC_URL" "${CCTP_STELLAR_RPC_URL}"
fi
if [[ -n "${CCTP_IRIS_BASE_URL:-}" ]]; then
  upsert_env "CCTP_IRIS_BASE_URL" "${CCTP_IRIS_BASE_URL}"
fi

# Compose prefers the process environment over --env-file. Re-export after
# upsert so a previously `source`d stale Sepolia URL cannot win.
export CCTP_ENABLED=true
export CCTP_ACCESS_TOKEN_HMAC_KEY="${HMAC_KEY}"
export CCTP_SEPOLIA_RPC_URL="${SEPOLIA_RPC}"
# Drop empty optional overrides that would wipe container defaults.
if [[ -z "${CCTP_STELLAR_RPC_URL:-}" ]]; then
  unset CCTP_STELLAR_RPC_URL || true
fi
if [[ -z "${CCTP_IRIS_BASE_URL:-}" ]]; then
  unset CCTP_IRIS_BASE_URL || true
fi
# Prefer CCTP_SEPOLIA_RPC_URL; clear alias so it cannot shadow.
unset SEPOLIA_RPC_URL || true

echo "Host .env.prod CCTP flags (no secrets):"
grep -E '^(CCTP_ENABLED|CCTP_SEPOLIA_RPC_URL|CCTP_STELLAR_RPC_URL|CCTP_IRIS_BASE_URL)=' "${ENV_FILE}" \
  | sed -E 's/(CCTP_ACCESS_TOKEN_HMAC_KEY=).*/\1<redacted>/' || true
if grep -qE '^CCTP_ACCESS_TOKEN_HMAC_KEY=.+' "${ENV_FILE}"; then
  echo "CCTP_ACCESS_TOKEN_HMAC_KEY=<set>"
else
  echo "CCTP_ACCESS_TOKEN_HMAC_KEY=<missing>" >&2
fi

echo "Applying CCTP DDL before API recreate (includes Fast finality check)..."
bash "${ROOT}/deploy/aws/scripts/apply-cctp-ddl.sh"

echo "Recreating API with CCTP enabled..."
COMPOSE=(docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod)
# Clear stale name conflicts from overlapping deploy/enable SSM runs.
"${COMPOSE[@]}" stop api >/dev/null 2>&1 || true
docker rm -f stellarroute-api-1 >/dev/null 2>&1 || true
# Compose sometimes leaves a hashed temporary name after a failed recreate.
docker ps -a --format '{{.Names}}' | grep -E '_?stellarroute-api-1$' \
  | xargs -r docker rm -f >/dev/null 2>&1 || true
"${COMPOSE[@]}" up -d --force-recreate --no-deps api

API_PORT="${API_HOST_PORT:-8080}"
echo "Waiting for API health..."
for _ in $(seq 1 60); do
  if curl -fsS "http://127.0.0.1:${API_PORT}/health" >/dev/null \
    && curl -fsS "http://127.0.0.1:${API_PORT}/health/deps" >/dev/null; then
    echo "API healthy."
    break
  fi
  sleep 5
done

echo "Container CCTP env (no secrets):"
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod exec -T api \
  sh -c 'printf "CCTP_ENABLED=%s\nCCTP_SEPOLIA_RPC_URL=%s\nCCTP_STELLAR_RPC_URL=%s\nHMAC_SET=%s\n" \
    "${CCTP_ENABLED:-}" "${CCTP_SEPOLIA_RPC_URL:-}" "${CCTP_STELLAR_RPC_URL:-}" \
    "$([ -n "${CCTP_ACCESS_TOKEN_HMAC_KEY:-}" ] && echo yes || echo no)"' || true

if v2_json="$(curl -sf "http://127.0.0.1:${API_PORT}/api/v2" 2>/dev/null)"; then
  printf '%s' "${v2_json}" | python3 -c '
import json,sys
d=json.load(sys.stdin).get("data",{})
print("bridge_settlement_executable={}".format(d.get("bridge_settlement_executable")))
print("corridors={}".format(len(d.get("supported_corridors") or [])))
for c in d.get("supported_corridors") or []:
    print(" corridor direction={} executable={}".format(c.get("direction"), c.get("executable")))
'
else
  echo "WARN: /api/v2 not ready yet" >&2
fi

echo "Recent API CCTP logs:"
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod logs --tail=80 api 2>/dev/null \
  | grep -E 'CCTP|cctp|attestation|Iris|Sepolia' || echo "(no CCTP log lines)"
