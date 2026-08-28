#!/usr/bin/env bash
# Install Docker Engine + Compose plugin on Ubuntu EC2 for single-host staging.
# Run as root or via sudo on a fresh instance:
#   sudo bash deploy/aws/scripts/bootstrap-ec2.sh
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "Re-run with sudo: sudo bash $0" >&2
  exit 1
fi

export DEBIAN_FRONTEND=noninteractive

apt-get update -y
apt-get install -y ca-certificates curl gnupg git jq unzip

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

# Small ARM and burstable shapes benefit from swap during the first Rust build.
if [[ ! -f /swapfile ]]; then
  echo "Creating 4G swapfile for Docker/Rust builds..."
  fallocate -l 4G /swapfile || dd if=/dev/zero of=/swapfile bs=1M count=4096
  chmod 600 /swapfile
  mkswap /swapfile
  swapon /swapfile
  if ! grep -q '^/swapfile ' /etc/fstab; then
    echo '/swapfile none swap sw 0 0' >>/etc/fstab
  fi
fi

docker version
docker compose version
echo "Bootstrap complete. Next: clone repo, copy deploy/env.prod.example to .env.prod, deploy the compose stack, and install the systemd unit."