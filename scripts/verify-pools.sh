#!/bin/bash
# StellarRoute — Verify Pool Registration Against Router Contract
#
# Checks that every non-placeholder pool in config/pools-<network>.json
# is registered in the live router contract.  Exits non-zero if any pool
# is missing — designed to be used as a CI/CD gate after register-pools.sh.
#
# Usage:
#   ./scripts/verify-pools.sh --network testnet
#
# Output:
#   Human-readable log to stdout/stderr
#   Machine-readable JSON summary to logs/<network>-verify-pools-summary.json

set -euo pipefail
source "$(dirname "$0")/lib/common.sh"

parse_network_flag "$@"
ensure_soroban_cli
ensure_log_dir
configure_network

# ── Resolve contract ID ───────────────────────────────────────────────

CONTRACT_ID="${ROUTER_CONTRACT_ADDRESS:-${STELLARROUTE_TESTNET_ROUTER_ID:-${SOROBAN_CONTRACT_ID:-}}}"
if [[ -z "${CONTRACT_ID}" && -f "$(deployment_file)" ]]; then
    CONTRACT_ID="$(get_contract_id)"
fi

if [[ -z "${CONTRACT_ID}" ]]; then
    log_error "No router contract ID found."
    log_error "Set ROUTER_CONTRACT_ADDRESS or run deploy.sh first."
    exit 1
fi

# ── Load pool config ──────────────────────────────────────────────────

POOLS_FILE="${CONFIG_DIR}/pools-${NETWORK}.json"
if [[ ! -f "${POOLS_FILE}" ]]; then
    log_error "Pool config not found: ${POOLS_FILE}"
    exit 1
fi

POOL_COUNT=$(jq '.pools | length' "${POOLS_FILE}")
log_info "Verifying ${POOL_COUNT} pool(s) from ${POOLS_FILE}"
log_info "Router contract: ${CONTRACT_ID}"

# ── Verification loop ─────────────────────────────────────────────────

VERIFIED=0
MISSING=0
SKIPPED_PLACEHOLDER=0

declare -a SUMMARY_POOLS=()

for i in $(seq 0 $((POOL_COUNT - 1))); do
    POOL_NAME=$(jq -r ".pools[$i].name" "${POOLS_FILE}")
    POOL_ADDR=$(jq -r ".pools[$i].address" "${POOLS_FILE}")

    if [[ "${POOL_ADDR}" == PLACEHOLDER* ]]; then
        log_warn "[$((i + 1))/${POOL_COUNT}] Skipping placeholder: ${POOL_NAME}"
        SKIPPED_PLACEHOLDER=$((SKIPPED_PLACEHOLDER + 1))
        SUMMARY_POOLS+=("{\"name\":\"${POOL_NAME}\",\"address\":\"${POOL_ADDR}\",\"status\":\"skipped_placeholder\"}")
        continue
    fi

    log_info "[$((i + 1))/${POOL_COUNT}] Verifying: ${POOL_NAME} (${POOL_ADDR})"

    IS_REGISTERED=$(invoke_contract "${CONTRACT_ID}" "is_pool_registered" --pool "${POOL_ADDR}" 2>/dev/null || echo "false")

    if [[ "${IS_REGISTERED}" == "true" ]]; then
        log_ok "Registered: ${POOL_NAME}"
        VERIFIED=$((VERIFIED + 1))
        SUMMARY_POOLS+=("{\"name\":\"${POOL_NAME}\",\"address\":\"${POOL_ADDR}\",\"status\":\"registered\"}")
    else
        log_error "MISSING from router: ${POOL_NAME} (${POOL_ADDR})"
        MISSING=$((MISSING + 1))
        SUMMARY_POOLS+=("{\"name\":\"${POOL_NAME}\",\"address\":\"${POOL_ADDR}\",\"status\":\"missing\"}")
    fi
done

# ── On-chain pool count ───────────────────────────────────────────────

TOTAL_ON_CHAIN=$(invoke_contract "${CONTRACT_ID}" "get_pool_count" 2>/dev/null || echo "unknown")

# ── JSON summary ──────────────────────────────────────────────────────

SUMMARY_FILE="${LOG_DIR}/${NETWORK}-verify-pools-summary.json"
POOLS_JSON=$(printf '%s\n' "${SUMMARY_POOLS[@]}" | paste -sd ',' -)
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
PASS=$([[ ${MISSING} -eq 0 ]] && echo "true" || echo "false")

cat > "${SUMMARY_FILE}" <<JSON
{
  "timestamp": "${TIMESTAMP}",
  "network": "${NETWORK}",
  "contract_id": "${CONTRACT_ID}",
  "pass": ${PASS},
  "verified": ${VERIFIED},
  "missing": ${MISSING},
  "skipped_placeholder": ${SKIPPED_PLACEHOLDER},
  "total_on_chain": "${TOTAL_ON_CHAIN}",
  "pools": [${POOLS_JSON}]
}
JSON

log_ok "JSON summary written to ${SUMMARY_FILE}"

# ── Human summary ─────────────────────────────────────────────────────

echo ""
if [[ ${MISSING} -eq 0 ]]; then
    log_ok "===== POOL VERIFICATION PASSED ====="
else
    log_error "===== POOL VERIFICATION FAILED ====="
fi
log_ok "Verified:              ${VERIFIED}"
log_ok "Missing:               ${MISSING}"
log_ok "Skipped (placeholder): ${SKIPPED_PLACEHOLDER}"
log_ok "Total on-chain pools:  ${TOTAL_ON_CHAIN}"

# ── Exit code — non-zero means CI gate should block ───────────────────

if [[ ${MISSING} -gt 0 ]]; then
    log_error "Run ./scripts/register-pools.sh --network ${NETWORK} to register missing pools."
    exit 1
fi

if [[ $((VERIFIED + SKIPPED_PLACEHOLDER)) -eq 0 ]]; then
    log_error "No pools were verified (config may be empty or all placeholders)."
    exit 1
fi
