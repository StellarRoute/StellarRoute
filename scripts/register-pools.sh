#!/bin/bash
# StellarRoute — Register Liquidity Pools with Router Contract
# Usage: ./scripts/register-pools.sh --network testnet

set -euo pipefail
source "$(dirname "$0")/lib/common.sh"

parse_network_flag "$@"
ensure_soroban_cli
ensure_log_dir
configure_network

CONTRACT_ID="${STELLARROUTE_TESTNET_ROUTER_ID:-${SOROBAN_CONTRACT_ID:-}}"
if [[ -z "${CONTRACT_ID}" && -f "$(deployment_file)" ]]; then
    CONTRACT_ID="$(get_contract_id)"
fi

if [[ -z "${CONTRACT_ID}" ]]; then
    log_error "Missing STELLARROUTE_TESTNET_ROUTER_ID or SOROBAN_CONTRACT_ID or deployment artifact."
    exit 1
fi

POOLS_FILE="${CONFIG_DIR}/pools-${NETWORK}.json"

if [[ ! -f "${POOLS_FILE}" ]]; then
    log_error "Pool config not found: ${POOLS_FILE}"
    exit 1
fi

POOL_COUNT=$(jq '.pools | length' "${POOLS_FILE}")
log_info "Registering ${POOL_COUNT} pools on ${NETWORK} (contract: ${CONTRACT_ID})"

REGISTERED=0
FAILED=0
SKIPPED=0

for i in $(seq 0 $((POOL_COUNT - 1))); do
    POOL_NAME=$(jq -r ".pools[$i].name" "${POOLS_FILE}")
    POOL_ADDR=$(jq -r ".pools[$i].address" "${POOLS_FILE}")

    if [[ "${POOL_ADDR}" == PLACEHOLDER* ]]; then
        log_warn "Skipping placeholder pool: ${POOL_NAME}"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    log_info "[$((i + 1))/${POOL_COUNT}] Registering: ${POOL_NAME} (${POOL_ADDR})"

    if invoke_contract "${CONTRACT_ID}" "register_pool" --pool "${POOL_ADDR}"; then
        log_tx "${POOL_ADDR}" "register_pool"

        IS_REGISTERED=$(invoke_contract "${CONTRACT_ID}" "is_pool_registered" --pool "${POOL_ADDR}")
        if [[ "${IS_REGISTERED}" == "true" ]]; then
            log_ok "Verified: ${POOL_NAME} is registered"
            REGISTERED=$((REGISTERED + 1))
        else
            log_error "Verification FAILED for ${POOL_NAME}"
            FAILED=$((FAILED + 1))
        fi
    else
        log_error "Registration FAILED for ${POOL_NAME}"
        FAILED=$((FAILED + 1))
    fi
done

TOTAL_POOLS=$(invoke_contract "${CONTRACT_ID}" "get_pool_count")

echo ""
log_ok "===== POOL REGISTRATION COMPLETE ====="
log_ok "Registered: ${REGISTERED}"
log_ok "Failed:     ${FAILED}"
log_ok "Skipped:    ${SKIPPED}"
log_ok "Total on-chain pool count: ${TOTAL_POOLS}"

if [[ ${FAILED} -gt 0 ]]; then
    exit 1
fi

if [[ ${REGISTERED} -eq 0 ]]; then
    log_error "No pools registered (all were skipped as placeholders)"
    exit 1
fi
