#!/usr/bin/env bash
# Opt-in read-only live attester enumeration (Sepolia + Stellar testnet).
# Records threshold, enabled count, and set hashes only — no secrets.
#
# Environment overrides (optional):
#   SEPOLIA_RPC_URL  — default https://ethereum-sepolia-rpc.publicnode.com
#   STELLAR_RPC_URL  — default https://soroban-testnet.stellar.org
#   CCTP_ENABLED     — forced false for this script
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"

export CCTP_ENABLED=false
export SEPOLIA_RPC_URL="${SEPOLIA_RPC_URL:-https://ethereum-sepolia-rpc.publicnode.com}"
export STELLAR_RPC_URL="${STELLAR_RPC_URL:-https://soroban-testnet.stellar.org}"

run_live_test() {
  local filter="$1"
  cargo test -p stellarroute-api --lib "$filter" -- --ignored --nocapture
}

run_live_test cctp::evm_attester_reader::live_tests::live_sepolia_enumeration
run_live_test cctp::stellar_attester_reader::live_tests::live_stellar_enumeration
