#!/bin/bash
# StellarRoute — Upgrade Contract on Stellar Network
# Usage: ./scripts/upgrade.sh --network testnet

set -euo pipefail
source "$(dirname "$0")/lib/common.sh"
trap 'trap_with_context ${LINENO} $?' ERR

parse_network_flag "$@"
ensure_soroban_cli
ensure_log_dir
configure_network

ROUTER_ID=$(get_named_contract_id "router")
ADAPTER_ID=$(get_named_contract_id "constant_product_adapter")

# ── Step 1: Capture pre-upgrade state ─────────────────────────────────

log_info "Capturing pre-upgrade state for router ${ROUTER_ID}..."
PRE_ADMIN=$(invoke_contract "${ROUTER_ID}" "get_admin" 2>/dev/null || echo "UNKNOWN")
PRE_FEE=$(invoke_contract "${ROUTER_ID}" "get_fee_rate_value" 2>/dev/null || echo "UNKNOWN")
PRE_PAUSED=$(invoke_contract "${ROUTER_ID}" "is_paused" 2>/dev/null || echo "UNKNOWN")
PRE_POOLS=$(invoke_contract "${ROUTER_ID}" "get_pool_count" 2>/dev/null || echo "UNKNOWN")
PRE_VERSION=$(invoke_contract "${ROUTER_ID}" "get_version" 2>/dev/null || echo "UNKNOWN")

log_info "Pre-upgrade: admin=${PRE_ADMIN} fee=${PRE_FEE} paused=${PRE_PAUSED} pools=${PRE_POOLS} version=${PRE_VERSION}"

# ── Step 2: Build new WASM ────────────────────────────────────────────

build_wasm
optimize_wasm

NEW_HASH=$(local_wasm_hash)
log_info "New WASM hash: ${NEW_HASH}"

# ── Step 3: Compare with deployed bytecode ────────────────────────────

log_info "Fetching deployed router bytecode hash..."
DEPLOYED_HASH=$(soroban_cmd contract fetch \
    --id "${ROUTER_ID}" \
    --network "${NETWORK}" \
    --output-file /tmp/stellarroute-deployed.wasm 2>/dev/null && \
    sha256sum /tmp/stellarroute-deployed.wasm | awk '{print $1}' || echo "FETCH_FAILED")

if [[ "${DEPLOYED_HASH}" == "${NEW_HASH}" ]]; then
    log_warn "New WASM is identical to deployed version. Nothing to upgrade."
    exit 0
fi

log_info "Bytecodes differ — proceeding with upgrade"
log_info "  Deployed: ${DEPLOYED_HASH}"
log_info "  New:      ${NEW_HASH}"

# ── Step 4: Install WASM + execute router upgrade path ───────────────

if [[ "${DRY_RUN}" == "true" ]]; then
    NEW_WASM_HASH="dry-run-upgrade-${NETWORK}"
    log_info "[DRY-RUN] skipped install/propose of router upgrade"
else
    NEW_WASM_HASH=$(soroban_cmd contract install \
        --wasm "${WASM_FILE}" \
        --source "${IDENTITY}" \
        --network "${NETWORK}")
fi

log_tx "${NEW_WASM_HASH}" "install_wasm"
log_ok "New WASM installed: ${NEW_WASM_HASH}"

if [[ "${DRY_RUN}" == "false" ]]; then
    EXECUTE_AFTER=$((0 + 4320))
    log_info "Submitting timelocked router upgrade proposal..."
    invoke_contract "${ROUTER_ID}" "propose_upgrade" \
        --admin "$(soroban_cmd keys address "${IDENTITY}")" \
        --new_wasm_hash "${NEW_WASM_HASH}" \
        --execute_after "${EXECUTE_AFTER}" >/dev/null
    log_ok "Upgrade proposal submitted for router ${ROUTER_ID}"
fi

if [[ "${DRY_RUN}" == "true" ]]; then
    log_info "[DRY-RUN] skipped adapter redeploy"
    NEW_ADAPTER_ID="dry-run-adapter-${NETWORK}"
else
    log_info "Redeploying adapter contract with latest WASM..."
    NEW_ADAPTER_ID="$(soroban_cmd contract deploy \
        --wasm "${WASM_FILE}" \
        --source "${IDENTITY}" \
        --network "${NETWORK}")"
    log_tx "${NEW_ADAPTER_ID}" "deploy_constant_product_adapter"
fi

# ── Step 5: Verify post-upgrade state ─────────────────────────────────

log_info "Verifying post-upgrade state..."
POST_ADMIN=$(invoke_contract "${ROUTER_ID}" "get_admin" 2>/dev/null || echo "UNKNOWN")
POST_FEE=$(invoke_contract "${ROUTER_ID}" "get_fee_rate_value" 2>/dev/null || echo "UNKNOWN")
POST_PAUSED=$(invoke_contract "${ROUTER_ID}" "is_paused" 2>/dev/null || echo "UNKNOWN")
POST_POOLS=$(invoke_contract "${ROUTER_ID}" "get_pool_count" 2>/dev/null || echo "UNKNOWN")
POST_VERSION=$(invoke_contract "${ROUTER_ID}" "get_version" 2>/dev/null || echo "UNKNOWN")

ERRORS=0

check_invariant() {
    local name="$1" pre="$2" post="$3"
    if [[ "${pre}" != "${post}" && "${pre}" != "UNKNOWN" ]]; then
        log_error "INVARIANT BROKEN: ${name} changed from '${pre}' to '${post}'"
        ERRORS=$((ERRORS + 1))
    else
        log_ok "${name}: ${post}"
    fi
}

check_invariant "admin"      "${PRE_ADMIN}"  "${POST_ADMIN}"
check_invariant "fee_rate"   "${PRE_FEE}"    "${POST_FEE}"
check_invariant "paused"     "${PRE_PAUSED}" "${POST_PAUSED}"
check_invariant "pool_count" "${PRE_POOLS}"  "${POST_POOLS}"

log_info "Version: ${PRE_VERSION} -> ${POST_VERSION}"

if [[ ${ERRORS} -gt 0 ]]; then
    log_error "Upgrade verification FAILED with ${ERRORS} broken invariants."
    exit 1
fi

UPDATED_DEPLOYMENT_CONTRACTS_JSON=$(cat <<JSON
{
  "router": {
    "contract_id": "${ROUTER_ID}",
    "wasm_path": "${WASM_FILE}",
    "pending_wasm_hash": "${NEW_WASM_HASH}"
  },
  "constant_product_adapter": {
    "contract_id": "${NEW_ADAPTER_ID:-${ADAPTER_ID}}",
    "wasm_path": "${WASM_FILE}"
  }
}
JSON
)
save_deployment "${UPDATED_DEPLOYMENT_CONTRACTS_JSON}"

echo ""
log_ok "===== UPGRADE COMPLETE ====="
log_ok "Router:   ${ROUTER_ID}"
log_ok "Adapter:  ${NEW_ADAPTER_ID:-${ADAPTER_ID}}"
log_ok "Version:  ${POST_VERSION}"
log_ok "Dry Run:  ${DRY_RUN}"
