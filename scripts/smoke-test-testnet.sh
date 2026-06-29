#!/bin/bash
# StellarRoute — Testnet contract smoke tests
# Usage: STELLARROUTE_TESTNET_ROUTER_ID=C... STELLARROUTE_SMOKE_ROUTE='...' ./scripts/smoke-test-testnet.sh --network testnet
# Optional pause/unpause coverage: STELLARROUTE_SMOKE_PAUSE=1 ./scripts/smoke-test-testnet.sh --network testnet

set -euo pipefail
source "$(dirname "$0")/lib/common.sh"
trap 'trap_with_context ${LINENO} $?' ERR

parse_network_flag "$@"
ensure_soroban_cli
configure_network

CONTRACT_ID="${STELLARROUTE_TESTNET_ROUTER_ID:-${SOROBAN_CONTRACT_ID:-}}"
if [[ -z "${CONTRACT_ID}" && -f "$(deployment_file)" ]]; then
    CONTRACT_ID="$(get_contract_id)"
fi

if [[ -z "${CONTRACT_ID}" ]]; then
    log_error "Missing STELLARROUTE_TESTNET_ROUTER_ID or SOROBAN_CONTRACT_ID."
    exit 1
fi

SMOKE_ROUTE="${STELLARROUTE_SMOKE_ROUTE:-}"
SMOKE_AMOUNT_IN="${STELLARROUTE_SMOKE_AMOUNT_IN:-10000000}"
SMOKE_PAUSE="${STELLARROUTE_SMOKE_PAUSE:-}"
IDENTITY="${IDENTITY:-deployer}"

if [[ -z "${SMOKE_ROUTE}" ]]; then
    log_error "Missing STELLARROUTE_SMOKE_ROUTE."
    exit 1
fi

log_info "Running testnet smoke tests against ${CONTRACT_ID}"

log_info "Checking pool count..."
POOL_COUNT=$(soroban_cmd contract invoke \
    --id "${CONTRACT_ID}" \
    --network "${NETWORK}" \
    -- get_pool_count)
if [[ "${POOL_COUNT}" -lt 1 ]]; then
    log_error "Expected at least 1 registered pool, got ${POOL_COUNT}"
    exit 1
fi
log_ok "Pool count check passed (${POOL_COUNT} pools registered)"

log_info "Calling validate_route..."
soroban_cmd contract invoke \
    --id "${CONTRACT_ID}" \
    --network "${NETWORK}" \
    -- validate_route \
    --route "${SMOKE_ROUTE}"
log_ok "validate_route passed"

log_info "Calling get_quote..."
soroban_cmd contract invoke \
    --id "${CONTRACT_ID}" \
    --network "${NETWORK}" \
    -- get_quote \
    --amount_in "${SMOKE_AMOUNT_IN}" \
    --route "${SMOKE_ROUTE}"
log_ok "get_quote passed"

# Optional pause/unpause smoke coverage
if [[ -n "${SMOKE_PAUSE}" ]]; then
    log_info "Running pause/unpause smoke coverage (STELLARROUTE_SMOKE_PAUSE=1)"
    log_info "Checking current pause status..."
    IS_PAUSED=$(soroban_cmd contract invoke \
        --id "${CONTRACT_ID}" \
        --network "${NETWORK}" \
        -- is_paused)
    log_ok "Current pause status: ${IS_PAUSED}"

    if [[ "${IS_PAUSED}" == "false" ]]; then
        log_info "Calling pause (using identity ${IDENTITY})..."
        soroban_cmd contract invoke \
            --id "${CONTRACT_ID}" \
            --source "${IDENTITY}" \
            --network "${NETWORK}" \
            -- pause
        log_ok "pause passed"

        log_info "Verifying contract is paused..."
        IS_PAUSED_AFTER=$(soroban_cmd contract invoke \
            --id "${CONTRACT_ID}" \
            --network "${NETWORK}" \
            -- is_paused)
        if [[ "${IS_PAUSED_AFTER}" != "true" ]]; then
            log_error "Contract not paused after pause call"
            exit 1
        fi
        log_ok "Contract is paused"

        log_info "Calling unpause (using identity ${IDENTITY})..."
        soroban_cmd contract invoke \
            --id "${CONTRACT_ID}" \
            --source "${IDENTITY}" \
            --network "${NETWORK}" \
            -- unpause
        log_ok "unpause passed"

        log_info "Verifying contract is unpaused..."
        IS_PAUSED_FINAL=$(soroban_cmd contract invoke \
            --id "${CONTRACT_ID}" \
            --network "${NETWORK}" \
            -- is_paused)
        if [[ "${IS_PAUSED_FINAL}" != "false" ]]; then
            log_error "Contract not unpaused after unpause call"
            exit 1
        fi
        log_ok "Contract is unpaused"
    else
        log_warn "Contract is already paused, skipping pause/unpause smoke coverage"
    fi
fi

log_ok "Testnet smoke tests passed"
