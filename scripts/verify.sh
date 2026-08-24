#!/bin/bash
# StellarRoute — Verify Deployed Contract Against Local Source
# Usage: ./scripts/verify.sh --network testnet

set -euo pipefail
source "$(dirname "$0")/lib/common.sh"

parse_network_flag "$@"
ensure_soroban_cli
ensure_log_dir
configure_network

CONTRACT_ID=$(get_contract_id)

echo ""
log_info "===== CONTRACT VERIFICATION ====="
log_info "Network:  ${NETWORK}"
log_info "Contract: ${CONTRACT_ID}"
echo ""

ERRORS=0

# ── Step 1: Bytecode Verification ─────────────────────────────────────

log_info "--- Bytecode Verification ---"

build_wasm
optimize_wasm

LOCAL_HASH=$(local_wasm_hash)
log_info "Local WASM SHA-256: ${LOCAL_HASH}"

DEPLOYED_WASM="/tmp/stellarroute-deployed-${NETWORK}.wasm"
log_info "Fetching deployed bytecode..."

if soroban_cmd contract fetch \
    --id "${CONTRACT_ID}" \
    --network "${NETWORK}" \
    --output-file "${DEPLOYED_WASM}" 2>/dev/null; then

    DEPLOYED_HASH=$(sha256sum "${DEPLOYED_WASM}" | awk '{print $1}')
    log_info "Deployed WASM SHA-256: ${DEPLOYED_HASH}"

    if [[ "${LOCAL_HASH}" == "${DEPLOYED_HASH}" ]]; then
        log_ok "Bytecode: MATCH"
    else
        log_error "Bytecode: MISMATCH"
        log_error "  Local:    ${LOCAL_HASH}"
        log_error "  Deployed: ${DEPLOYED_HASH}"
        ERRORS=$((ERRORS + 1))
    fi
else
    log_error "Failed to fetch deployed bytecode"
    ERRORS=$((ERRORS + 1))
fi

# ── Step 2: State Verification ────────────────────────────────────────

echo ""
log_info "--- State Verification ---"

verify_state() {
    local label="$1"
    local fn_name="$2"
    shift 2

    local result
    if result=$(invoke_contract "${CONTRACT_ID}" "${fn_name}" "$@" 2>/dev/null); then
        log_ok "${label}: ${result}"
    else
        log_error "${label}: CALL FAILED"
        ERRORS=$((ERRORS + 1))
    fi
}

verify_state "Admin"      "get_admin"
verify_state "Fee Rate"   "get_fee_rate_value"
verify_state "Fee To"     "get_fee_to"
verify_state "Paused"     "is_paused"
verify_state "Pool Count" "get_pool_count"
verify_state "Version"    "version"

# ── Summary ───────────────────────────────────────────────────────────

echo ""
if [[ ${ERRORS} -eq 0 ]]; then
    log_ok "===== VERIFICATION PASSED ====="
else
    log_error "===== VERIFICATION FAILED (${ERRORS} errors) ====="
    exit 1
fi
