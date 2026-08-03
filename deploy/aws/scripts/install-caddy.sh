#!/usr/bin/env bash
# Install Caddy on Ubuntu and configure HTTPS reverse proxy for the staging API.
# Example:
#   sudo bash deploy/aws/scripts/install-caddy.sh api.example.com ops@example.com
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "Re-run with sudo: sudo bash $0 <domain> [email]" >&2
  exit 1
fi

DOMAIN="${1:-}"
EMAIL="${2:-}"

if [[ -z "${DOMAIN}" ]]; then
  echo "Usage: sudo bash $0 <domain> [email]" >&2
  exit 1
fi

apt-get update -y
apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl gnupg

curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \
  | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \
  | tee /etc/apt/sources.list.d/caddy-stable.list >/dev/null
chmod o+r /usr/share/keyrings/caddy-stable-archive-keyring.gpg
chmod o+r /etc/apt/sources.list.d/caddy-stable.list

apt-get update -y
apt-get install -y caddy

if [[ -n "${EMAIL}" ]]; then
  cat >/etc/caddy/Caddyfile <<EOF
{
  email ${EMAIL}
}

${DOMAIN} {
  encode zstd gzip
  reverse_proxy 127.0.0.1:8080
}
EOF
else
  cat >/etc/caddy/Caddyfile <<EOF
${DOMAIN} {
  encode zstd gzip
  reverse_proxy 127.0.0.1:8080
}
EOF
fi

caddy fmt --overwrite /etc/caddy/Caddyfile
systemctl enable --now caddy
systemctl reload caddy
systemctl status caddy --no-pager

echo "Caddy configured for https://${DOMAIN} -> http://127.0.0.1:8080"
echo "Ensure DNS for ${DOMAIN} points to this instance and ports 80/443 are open."