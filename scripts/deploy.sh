#!/bin/bash
# StellarRoute — Deploy Contract to Stellar Network
# Usage: ./scripts/deploy.sh --network testnet

set -euo pipefail
source "$(dirname "$0")/lib/common.sh"
trap 'trap_with_context ${LINENO} $?' ERR

parse_network_flag "$@"
ensure_soroban_cli
ensure_log_dir
configure_network

# ── Step 1: Build ─────────────────────────────────────────────────────

build_wasm
optimize_wasm

# ── Step 2: Deploy contracts ──────────────────────────────────────────

declare -A DEPLOYED_IDS
declare -A CONTRACT_WASM
CONTRACT_WASM["router"]="${WASM_FILE}"
CONTRACT_WASM["constant_product_adapter"]="${WASM_FILE}"

for CONTRACT_NAME in "router" "constant_product_adapter"; do
    log_info "Deploying ${CONTRACT_NAME} to ${NETWORK}..."
    if [[ "${DRY_RUN}" == "true" ]]; then
        DEPLOYED_IDS["${CONTRACT_NAME}"]="dry-run-${CONTRACT_NAME}-${NETWORK}"
        log_info "[DRY-RUN] skipped on-chain deploy for ${CONTRACT_NAME}"
        continue
    fi

    CONTRACT_ID="$(soroban_cmd contract deploy \
        --wasm "${CONTRACT_WASM[${CONTRACT_NAME}]}" \
        --source "${IDENTITY}" \
        --network "${NETWORK}" \
        --network-passphrase "$(get_network_passphrase)" \
        --rpc-url "$(get_rpc_url)")"
    DEPLOYED_IDS["${CONTRACT_NAME}"]="${CONTRACT_ID}"
    log_ok "Contract deployed (${CONTRACT_NAME}): ${CONTRACT_ID}"
    log_tx "${CONTRACT_ID}" "deploy_${CONTRACT_NAME}"
done

# ── Step 3: Initialize router ─────────────────────────────────────────

if [[ "${DRY_RUN}" == "true" ]]; then
    ADMIN_ADDRESS="dry-run-admin"
    FEE_TO="dry-run-fee-to"
else
    ADMIN_ADDRESS=$(soroban_cmd keys address "${IDENTITY}")
    # Contract rejects admin == fee_to (InvalidAmount). Prefer a dedicated fee identity.
    if soroban_cmd keys address fee_to >/dev/null 2>&1; then
        FEE_TO=$(soroban_cmd keys address fee_to)
    else
        log_info "Generating distinct fee_to identity (admin and fee_to must differ)..."
        soroban_cmd keys generate fee_to --network "${NETWORK}" >/dev/null
        FEE_TO=$(soroban_cmd keys address fee_to)
        # Fund fee_to on testnet so it can hold XLM if needed later (not required for init).
        if [[ "${NETWORK}" == "testnet" ]]; then
            curl -fsS "https://friendbot.stellar.org/?addr=${FEE_TO}" >/dev/null || true
        fi
    fi
    if [[ "${ADMIN_ADDRESS}" == "${FEE_TO}" ]]; then
        log_error "admin and fee_to must be distinct Stellar addresses"
        exit 1
    fi
fi
FEE_RATE=30
ROUTER_ID="${DEPLOYED_IDS[router]}"

if [[ "${DRY_RUN}" == "true" ]]; then
    log_info "[DRY-RUN] skipped initialize for router ${ROUTER_ID}"
else
    log_info "Initializing router (admin=${ADMIN_ADDRESS}, fee_rate=${FEE_RATE}, fee_to=${FEE_TO})..."
    invoke_contract "${ROUTER_ID}" "initialize" \
        --admin "${ADMIN_ADDRESS}" \
        --fee_rate "${FEE_RATE}" \
        --fee_to "${FEE_TO}"
fi

log_ok "Router initialization step complete"

# ── Step 4: Save Deployment Artifact ──────────────────────────────────

DEPLOYMENT_CONTRACTS_JSON=$(cat <<JSON
{
  "router": {
    "contract_id": "${DEPLOYED_IDS[router]}",
    "wasm_path": "${WASM_FILE}"
  },
  "constant_product_adapter": {
    "contract_id": "${DEPLOYED_IDS[constant_product_adapter]}",
    "wasm_path": "${WASM_FILE}"
  }
}
JSON
)
save_deployment "${DEPLOYMENT_CONTRACTS_JSON}"
save_public_deployment "${DEPLOYED_IDS[router]}" "${DEPLOYED_IDS[constant_product_adapter]}"

# ── Step 5: Verify Deployment ─────────────────────────────────────────

if [[ "${DRY_RUN}" == "true" ]]; then
    log_info "[DRY-RUN] skipped post-deploy verification"
else
    log_info "Verifying router deployment via get_admin()..."
    DEPLOYED_ADMIN=$(invoke_contract "${ROUTER_ID}" "get_admin")

    if [[ "${DEPLOYED_ADMIN}" == *"${ADMIN_ADDRESS}"* ]]; then
        log_ok "Deployment verified: admin matches"
    else
        log_error "Deployment verification FAILED: expected ${ADMIN_ADDRESS}, got ${DEPLOYED_ADMIN}"
        exit 1
    fi
fi

echo ""
log_ok "===== DEPLOYMENT COMPLETE ====="
log_ok "Network:     ${NETWORK}"
log_ok "Router ID:   ${DEPLOYED_IDS[router]}"
log_ok "Adapter ID:  ${DEPLOYED_IDS[constant_product_adapter]}"
log_ok "Admin:       ${ADMIN_ADDRESS}"
log_ok "Fee Rate:    ${FEE_RATE} bps"
log_ok "Dry Run:     ${DRY_RUN}"
log_ok "Artifact:    $(deployment_file)"
log_ok "Public:      $(public_deployment_file)"
echo ""
log_info "Set ROUTER_CONTRACT_ADDRESS from the committed artifact:"
log_info "  export ROUTER_CONTRACT_ADDRESS=\"\$(jq -r .router_contract_id $(public_deployment_file))\""
