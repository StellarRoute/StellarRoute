#!/bin/bash
# StellarRoute — Idempotent Pool Registration with Router Contract
#
# Registers every pool in config/pools-<network>.json against the deployed
# router contract.  Re-running the script is safe: pools that are already
# registered are skipped without error.
#
# Usage:
#   ./scripts/register-pools.sh --network testnet
#   ./scripts/register-pools.sh --network testnet --dry-run
#
# Required env / deployment artifact:
#   ROUTER_CONTRACT_ADDRESS  — or STELLARROUTE_TESTNET_ROUTER_ID / SOROBAN_CONTRACT_ID
#   config/deployment-testnet.json  — written by deploy.sh
#
# Output:
#   Human-readable log to stdout/stderr
#   Machine-readable JSON summary to logs/<network>-register-summary.json

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
    log_error "Set ROUTER_CONTRACT_ADDRESS, STELLARROUTE_TESTNET_ROUTER_ID,"
    log_error "or SOROBAN_CONTRACT_ID, or run deploy.sh first."
    exit 1
fi

# ── Load pool config ──────────────────────────────────────────────────

POOLS_FILE="${CONFIG_DIR}/pools-${NETWORK}.json"
if [[ ! -f "${POOLS_FILE}" ]]; then
    log_error "Pool config not found: ${POOLS_FILE}"
    exit 1
fi

POOL_COUNT=$(jq '.pools | length' "${POOLS_FILE}")
log_info "Processing ${POOL_COUNT} pool(s) from ${POOLS_FILE}"
log_info "Router contract: ${CONTRACT_ID}"

# ── Registration loop ─────────────────────────────────────────────────

REGISTERED=0
SKIPPED_PLACEHOLDER=0
ALREADY_REGISTERED=0
FAILED=0

declare -a SUMMARY_POOLS=()

for i in $(seq 0 $((POOL_COUNT - 1))); do
    POOL_NAME=$(jq -r ".pools[$i].name" "${POOLS_FILE}")
    POOL_ADDR=$(jq -r ".pools[$i].address" "${POOLS_FILE}")

    # Skip placeholder addresses
    if [[ "${POOL_ADDR}" == PLACEHOLDER* ]]; then
        log_warn "[$((i + 1))/${POOL_COUNT}] Skipping placeholder: ${POOL_NAME}"
        SKIPPED_PLACEHOLDER=$((SKIPPED_PLACEHOLDER + 1))
        SUMMARY_POOLS+=("{\"name\":\"${POOL_NAME}\",\"address\":\"${POOL_ADDR}\",\"status\":\"skipped_placeholder\"}")
        continue
    fi

    log_info "[$((i + 1))/${POOL_COUNT}] Checking: ${POOL_NAME} (${POOL_ADDR})"

    # ── Idempotency check: is the pool already registered? ────────────
    ALREADY=""
    if ! ALREADY=$(invoke_contract "${CONTRACT_ID}" "is_pool_registered" --pool "${POOL_ADDR}" 2>/dev/null); then
        log_warn "Could not query is_pool_registered for ${POOL_NAME}; will attempt registration."
        ALREADY="false"
    fi

    if [[ "${ALREADY}" == "true" ]]; then
        log_ok "Already registered (no-op): ${POOL_NAME}"
        ALREADY_REGISTERED=$((ALREADY_REGISTERED + 1))
        SUMMARY_POOLS+=("{\"name\":\"${POOL_NAME}\",\"address\":\"${POOL_ADDR}\",\"status\":\"already_registered\"}")
        continue
    fi

    # ── Register ──────────────────────────────────────────────────────
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY-RUN] Would register: ${POOL_NAME} (${POOL_ADDR})"
        SUMMARY_POOLS+=("{\"name\":\"${POOL_NAME}\",\"address\":\"${POOL_ADDR}\",\"status\":\"dry_run\"}")
        continue
    fi

    if invoke_contract "${CONTRACT_ID}" "register_pool" --pool "${POOL_ADDR}" 2>&1 | tee -a "${LOG_DIR}/${NETWORK}-register.log"; then
        log_tx "${POOL_ADDR}" "register_pool"

        # Verify the registration took effect
        IS_REGISTERED=$(invoke_contract "${CONTRACT_ID}" "is_pool_registered" --pool "${POOL_ADDR}" 2>/dev/null || echo "false")
        if [[ "${IS_REGISTERED}" == "true" ]]; then
            log_ok "Registered and verified: ${POOL_NAME}"
            REGISTERED=$((REGISTERED + 1))
            SUMMARY_POOLS+=("{\"name\":\"${POOL_NAME}\",\"address\":\"${POOL_ADDR}\",\"status\":\"registered\"}")
        else
            log_error "Registration call succeeded but verification FAILED: ${POOL_NAME}"
            FAILED=$((FAILED + 1))
            SUMMARY_POOLS+=("{\"name\":\"${POOL_NAME}\",\"address\":\"${POOL_ADDR}\",\"status\":\"verify_failed\"}")
        fi
    else
        log_error "Registration FAILED for: ${POOL_NAME} (${POOL_ADDR})"
        FAILED=$((FAILED + 1))
        SUMMARY_POOLS+=("{\"name\":\"${POOL_NAME}\",\"address\":\"${POOL_ADDR}\",\"status\":\"register_failed\"}")
    fi
done

# ── On-chain pool count ───────────────────────────────────────────────

TOTAL_ON_CHAIN=0
if [[ "${DRY_RUN}" != "true" ]]; then
    TOTAL_ON_CHAIN=$(invoke_contract "${CONTRACT_ID}" "get_pool_count" 2>/dev/null || echo "0")
fi

# ── JSON summary ──────────────────────────────────────────────────────

SUMMARY_FILE="${LOG_DIR}/${NETWORK}-register-summary.json"
POOLS_JSON=$(printf '%s\n' "${SUMMARY_POOLS[@]}" | paste -sd ',' -)
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

cat > "${SUMMARY_FILE}" <<JSON
{
  "timestamp": "${TIMESTAMP}",
  "network": "${NETWORK}",
  "contract_id": "${CONTRACT_ID}",
  "dry_run": ${DRY_RUN},
  "registered": ${REGISTERED},
  "already_registered": ${ALREADY_REGISTERED},
  "skipped_placeholder": ${SKIPPED_PLACEHOLDER},
  "failed": ${FAILED},
  "total_on_chain": ${TOTAL_ON_CHAIN},
  "pools": [${POOLS_JSON}]
}
JSON

log_ok "JSON summary written to ${SUMMARY_FILE}"

# ── Human summary ─────────────────────────────────────────────────────

echo ""
log_ok "===== POOL REGISTRATION COMPLETE ====="
log_ok "Registered (new):     ${REGISTERED}"
log_ok "Already registered:   ${ALREADY_REGISTERED}"
log_ok "Skipped (placeholder):${SKIPPED_PLACEHOLDER}"
log_ok "Failed:               ${FAILED}"
if [[ "${DRY_RUN}" != "true" ]]; then
    log_ok "Total on-chain pools: ${TOTAL_ON_CHAIN}"
fi

# ── Exit code ─────────────────────────────────────────────────────────

if [[ ${FAILED} -gt 0 ]]; then
    log_error "One or more registrations failed — see ${SUMMARY_FILE}"
    exit 1
fi

# In a non-dry-run, at least one pool should have been processed (registered
# or already registered). If every pool was a placeholder, warn but succeed.
if [[ "${DRY_RUN}" != "true" && $((REGISTERED + ALREADY_REGISTERED)) -eq 0 && ${SKIPPED_PLACEHOLDER} -gt 0 ]]; then
    log_warn "All pools were placeholders. Update config/pools-${NETWORK}.json with real addresses."
fi
