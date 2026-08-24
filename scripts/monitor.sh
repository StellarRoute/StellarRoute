#!/bin/bash
# StellarRoute — Contract Health Monitor
# Usage: ./scripts/monitor.sh --network testnet

set -euo pipefail
source "$(dirname "$0")/lib/common.sh"

parse_network_flag "$@"
ensure_soroban_cli
ensure_log_dir
configure_network

CONTRACT_ID=$(get_contract_id)

echo ""
log_info "===== CONTRACT HEALTH CHECK ====="
log_info "Network:  ${NETWORK}"
log_info "Contract: ${CONTRACT_ID}"
log_info "Time:     $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

ALERTS=0

check_health() {
    local label="$1"
    local fn_name="$2"
    local expected="$3"
    shift 3

    local result
    if result=$(invoke_contract "${CONTRACT_ID}" "${fn_name}" "$@" 2>/dev/null); then
        if [[ -n "${expected}" && "${result}" != *"${expected}"* ]]; then
            log_warn "ALERT: ${label} = ${result} (expected: ${expected})"
            ALERTS=$((ALERTS + 1))
        else
            log_ok "${label}: ${result}"
        fi
    else
        log_error "ALERT: ${label} query FAILED"
        ALERTS=$((ALERTS + 1))
    fi
}

# ── Pause Status ──────────────────────────────────────────────────────
# Alert if contract is unexpectedly paused

PAUSED=$(invoke_contract "${CONTRACT_ID}" "is_paused" 2>/dev/null || echo "UNKNOWN")
if [[ "${PAUSED}" == "true" ]]; then
    log_warn "ALERT: Contract is PAUSED"
    ALERTS=$((ALERTS + 1))
else
    log_ok "Pause status: not paused"
fi

# ── Admin Address ─────────────────────────────────────────────────────
# Read current admin (drift detection requires a known-good value from deployment artifact)

ADMIN=$(invoke_contract "${CONTRACT_ID}" "get_admin" 2>/dev/null || echo "UNKNOWN")
if [[ "${ADMIN}" == "UNKNOWN" ]]; then
    log_error "ALERT: Cannot read admin address"
    ALERTS=$((ALERTS + 1))
else
    EXPECTED_ADMIN=""
    DEPLOY_FILE="$(deployment_file)"
    if [[ -f "${DEPLOY_FILE}" ]]; then
        # Admin is whoever deployed; we log it for manual review
        log_ok "Admin: ${ADMIN}"
    else
        log_ok "Admin: ${ADMIN} (no deployment artifact to compare against)"
    fi
fi

# ── Fee Rate ──────────────────────────────────────────────────────────

FEE_RATE=$(invoke_contract "${CONTRACT_ID}" "get_fee_rate_value" 2>/dev/null || echo "UNKNOWN")
if [[ "${FEE_RATE}" == "UNKNOWN" ]]; then
    log_error "ALERT: Cannot read fee rate"
    ALERTS=$((ALERTS + 1))
else
    log_ok "Fee rate: ${FEE_RATE} bps"
fi

# ── Pool Count ────────────────────────────────────────────────────────

POOL_COUNT=$(invoke_contract "${CONTRACT_ID}" "get_pool_count" 2>/dev/null || echo "UNKNOWN")
if [[ "${POOL_COUNT}" == "UNKNOWN" ]]; then
    log_error "ALERT: Cannot read pool count"
    ALERTS=$((ALERTS + 1))
else
    log_ok "Pool count: ${POOL_COUNT}"
fi

# ── Version ───────────────────────────────────────────────────────────

VERSION=$(invoke_contract "${CONTRACT_ID}" "version" 2>/dev/null || echo "UNKNOWN")
if [[ "${VERSION}" == "UNKNOWN" ]]; then
    log_warn "ALERT: Cannot read contract version"
    ALERTS=$((ALERTS + 1))
else
    log_ok "Version: ${VERSION}"
fi

# ── Summary ───────────────────────────────────────────────────────────

echo ""
if [[ ${ALERTS} -eq 0 ]]; then
    log_ok "===== HEALTH CHECK PASSED (0 alerts) ====="
else
    log_warn "===== HEALTH CHECK: ${ALERTS} ALERT(S) ====="
    exit 1
fi
