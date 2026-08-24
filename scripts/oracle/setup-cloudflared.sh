#!/usr/bin/env bash
# scripts/oracle/setup-cloudflared.sh
# Install guidance + write a tunnel config targeting the local API.
# Named tunnel credentials still require `cloudflared tunnel login` interactively.
#
# Usage:
#   TUNNEL_HOSTNAME=api.example.com ./scripts/oracle/setup-cloudflared.sh
set -euo pipefail

HOSTNAME="${TUNNEL_HOSTNAME:-}"
API_PORT="${API_HOST_PORT:-8080}"
CONFIG_DIR="${HOME}/.cloudflared"
EXAMPLE_SRC="$(cd "$(dirname "$0")/../.." && pwd)/deploy/cloudflared/config.example.yml"

if ! command -v cloudflared >/dev/null 2>&1; then
  echo "cloudflared not found. Install:" >&2
  echo "  https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/install-and-setup/installation/" >&2
  echo "  macOS: brew install cloudflare/cloudflare/cloudflared" >&2
  exit 1
fi

if [[ -z "${HOSTNAME}" ]]; then
  echo "Set TUNNEL_HOSTNAME=api.yourdomain.com" >&2
  exit 1
fi

mkdir -p "${CONFIG_DIR}"
if [[ ! -f "${CONFIG_DIR}/config.yml" ]]; then
  cp "${EXAMPLE_SRC}" "${CONFIG_DIR}/config.yml"
  echo "Wrote ${CONFIG_DIR}/config.yml from example — replace TUNNEL_UUID and hostname."
fi

# Patch hostname + local service port if placeholders remain.
if grep -q 'api.example.com' "${CONFIG_DIR}/config.yml" 2>/dev/null; then
  sed -i.bak "s|api.example.com|${HOSTNAME}|g" "${CONFIG_DIR}/config.yml"
  sed -i.bak "s|127.0.0.1:8080|127.0.0.1:${API_PORT}|g" "${CONFIG_DIR}/config.yml"
  rm -f "${CONFIG_DIR}/config.yml.bak"
fi

cat <<EOF
Next steps (interactive — run on the Oracle VM):
  1. cloudflared tunnel login
  2. cloudflared tunnel create stellarroute-api
  3. Put the tunnel UUID + credentials path into ${CONFIG_DIR}/config.yml
  4. cloudflared tunnel route dns stellarroute-api ${HOSTNAME}
  5. sudo cloudflared service install && sudo systemctl enable --now cloudflared
  6. curl -sf https://${HOSTNAME}/health

See docs/deployment/oracle-always-free.md
EOF
