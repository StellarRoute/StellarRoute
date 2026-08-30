#!/usr/bin/env bash
# scripts/oracle/bootstrap-vm.sh
# Install Docker Engine + Compose plugin on Ubuntu (Oracle Always Free ARM).
# Run as root or via sudo on a fresh VM:
#   sudo bash scripts/oracle/bootstrap-vm.sh
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "Re-run with sudo: sudo bash $0" >&2
  exit 1
fi

export DEBIAN_FRONTEND=noninteractive

apt-get update -y
apt-get install -y ca-certificates curl gnupg git jq

install -m 0755 -d /etc/apt/keyrings
if [[ ! -f /etc/apt/keyrings/docker.asc ]]; then
  curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
  chmod a+r /etc/apt/keyrings/docker.asc
fi

ARCH="$(dpkg --print-architecture)"
CODENAME="$(. /etc/os-release && echo "${VERSION_CODENAME}")"
echo \
  "deb [arch=${ARCH} signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu ${CODENAME} stable" \
  >/etc/apt/sources.list.d/docker.list

apt-get update -y
apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

systemctl enable --now docker

TARGET_USER="${SUDO_USER:-${USER}}"
if id "${TARGET_USER}" &>/dev/null; then
  usermod -aG docker "${TARGET_USER}"
  echo "Added ${TARGET_USER} to docker group (log out/in for group to apply)."
fi

# Optional swap helps first ARM cargo release builds on smaller shapes.
if [[ ! -f /swapfile ]]; then
  echo "Creating 8G swapfile for Docker/Rust builds…"
  fallocate -l 8G /swapfile || dd if=/dev/zero of=/swapfile bs=1M count=8192
  chmod 600 /swapfile
  mkswap /swapfile
  swapon /swapfile
  if ! grep -q '^/swapfile ' /etc/fstab; then
    echo '/swapfile none swap sw 0 0' >>/etc/fstab
  fi
fi

docker version
docker compose version
echo "Bootstrap complete. Next: clone repo, copy deploy/env.prod.example → .env.prod, compose up."
echo "See docs/deployment/oracle-always-free.md"
