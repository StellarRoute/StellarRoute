#!/bin/bash
# StellarRoute — Off-Chain TTL Extension Bot
#
# This script periodically checks the contract's TTL status and calls
# extend_storage_ttl() when storage keys are approaching expiry.
#
# Usage:
#   ./scripts/extend-ttl.sh --network testnet              # One-shot check & extend
#   ./scripts/extend-ttl.sh --network testnet --watch       # Continuous monitoring
#   ./scripts/extend-ttl.sh --network testnet --dry-run     # Check status only
#
# Prerequisites:
#   - Soroban CLI installed (soroban or stellar)
#   - A funded deployer identity configured
#   - Contract deployed (deployment artifact exists)
#
# Cost Analysis:
#   - extend_storage_ttl cost depends on number of registered pools.
#   - Each pool TTL extension costs ~100 stroops (0.00001 XLM).
#   - Instance TTL extension costs ~100 stroops.
#   - With 10 pools, estimated cost per call: ~0.0015 XLM.
#   - Recommended interval: weekly → ~0.006 XLM/month.
#   - Monthly budget for 100 pools: ~0.06 XLM.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

# ── Configuration ─────────────────────────────────────────────────────

WATCH_MODE=false
DRY_RUN=false
CHECK_INTERVAL=86400  # 24 hours in seconds (default watch interval)

# ── Parse Arguments ───────────────────────────────────────────────────

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --network)
                NETWORK="$2"
                shift 2
                ;;
            --watch)
                WATCH_MODE=true
                shift
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --interval)
                CHECK_INTERVAL="$2"
                shift 2
                ;;
            --identity)
                IDENTITY="$2"
                shift 2
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown argument: $1"
                usage
                exit 1
                ;;
        esac
    done

    if [[ -z "${NETWORK}" ]]; then
        log_error "Missing required flag: --network (testnet|mainnet)"
        exit 1
    fi

    if [[ "${NETWORK}" != "testnet" && "${NETWORK}" != "mainnet" ]]; then
        log_error "Invalid network '${NETWORK}'. Must be 'testnet' or 'mainnet'."
        exit 1
    fi
}

usage() {
    cat <<EOF
StellarRoute TTL Extension Bot

Usage: $(basename "$0") [OPTIONS]

Options:
  --network <net>     Network to use: testnet or mainnet (required)
  --watch             Run continuously, checking every --interval seconds
  --dry-run           Check TTL status without extending
  --interval <secs>   Watch interval in seconds (default: 86400 = 24h)
  --identity <name>   Soroban identity to use (default: deployer)
  -h, --help          Show this help message

Examples:
  $(basename "$0") --network testnet                    # One-shot extend
  $(basename "$0") --network testnet --dry-run          # Status check only
  $(basename "$0") --network testnet --watch            # Monitor every 24h
  $(basename "$0") --network mainnet --watch --interval 43200  # Every 12h

Cost Estimates:
  Per invocation:  ~0.001-0.01 XLM (depends on pool count)
  Weekly (10 pools): ~0.006 XLM/month
  Daily  (10 pools): ~0.04  XLM/month
EOF
}

# ── TTL Status Check ──────────────────────────────────────────────────

check_ttl_status() {
    local contract_id
    contract_id="$(get_contract_id)"

    log_info "Querying TTL status for contract ${contract_id} on ${NETWORK}..."

    local status
    status=$(invoke_contract "${contract_id}" "get_ttl_status" 2>&1) || {
        log_error "Failed to query TTL status: ${status}"
        return 1
    }

    echo "${status}"
}

parse_needs_extension() {
    local status_json="$1"
    # Parse the needs_extension field from the TTLStatus response
    echo "${status_json}" | grep -o '"needs_extension":[a-z]*' | cut -d: -f2
}

# ── TTL Extension ─────────────────────────────────────────────────────

extend_ttl() {
    local contract_id
    contract_id="$(get_contract_id)"

    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] Would call extend_storage_ttl on ${contract_id}"
        return 0
    fi

    log_info "Calling extend_storage_ttl on ${contract_id}..."

    local result
    result=$(invoke_contract "${contract_id}" "extend_storage_ttl" 2>&1) || {
        log_error "Failed to extend TTL: ${result}"
        # Alert: extension failed — this is critical
        alert_failure "${result}"
        return 1
    }

    log_ok "TTL extension successful"
    log_tx "$(echo "${result}" | head -1)" "extend_storage_ttl"
    return 0
}

# ── Alerting ──────────────────────────────────────────────────────────

redact_secrets() {
    local text="$1"
    # Redact any potential secrets (private keys, API keys, etc.)
    echo "${text}" | sed -E 's/(secret|key|token|password)=[[:alnum:]_-]+/\1=REDACTED/gI'
}

alert_failure() {
    local error_msg="$1"
    local timestamp
    timestamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    local contract_id
    contract_id="$(get_contract_id)"
    local redacted_error
    redacted_error="$(redact_secrets "${error_msg}")"

    log_error "ALERT: TTL extension failed! Contract storage may be at risk."
    log_error "Error: ${redacted_error}"
    log_error "Action required: manually extend TTLs or investigate."

    # Send webhook alert if configured
    if [[ -n "${TTL_ALERT_WEBHOOK_URL:-}" ]]; then
        log_info "Sending failure alert to webhook..."
        
        # Slack-compatible JSON payload
        local payload
        payload=$(cat <<EOF
{
  "text": "⚠️ StellarRoute TTL Extension Failed",
  "attachments": [
    {
      "color": "danger",
      "title": "TTL Extension Failure",
      "fields": [
        {
          "title": "Network",
          "value": "${NETWORK}",
          "short": true
        },
        {
          "title": "Contract ID",
          "value": "${contract_id}",
          "short": true
        },
        {
          "title": "Timestamp",
          "value": "${timestamp}",
          "short": true
        }
      ],
      "text": "Error: ${redacted_error}",
      "footer": "StellarRoute TTL Bot"
    }
  ]
}
EOF
)

        if command -v curl &>/dev/null; then
            curl -X POST "${TTL_ALERT_WEBHOOK_URL}" \
                -H "Content-Type: application/json" \
                -d "${payload}" 2>/dev/null || log_warn "Failed to send webhook alert"
        else
            log_warn "curl not available, cannot send webhook alert"
        fi
    fi
}

# ── Main Logic ────────────────────────────────────────────────────────

run_check_and_extend() {
    log_info "=== TTL Extension Check ($(date -u +%Y-%m-%dT%H:%M:%SZ)) ==="

    local status
    if status=$(check_ttl_status); then
        log_info "TTL Status: ${status}"

        local needs_ext
        needs_ext=$(parse_needs_extension "${status}")

        if [[ "${needs_ext}" == "true" ]]; then
            log_warn "TTL extension needed — storage keys approaching expiry"
            extend_ttl
        else
            log_ok "TTL status healthy — no extension needed"

            # Even if not needed based on estimate, extend if we haven't
            # extended recently (belt-and-suspenders approach)
            if [[ "${DRY_RUN}" != "true" ]]; then
                log_info "Performing preventive TTL extension anyway..."
                extend_ttl
            fi
        fi
    else
        log_error "Could not determine TTL status"
        # Try extending anyway as a safety measure
        if [[ "${DRY_RUN}" != "true" ]]; then
            log_warn "Attempting TTL extension despite status check failure..."
            extend_ttl
        fi
    fi

    log_info "=== Check complete ==="
}

main() {
    parse_args "$@"
    ensure_soroban_cli
    configure_network
    ensure_log_dir

    if [[ "${WATCH_MODE}" == "true" ]]; then
        log_info "Starting TTL extension bot in watch mode (interval: ${CHECK_INTERVAL}s)"
        while true; do
            run_check_and_extend || log_error "Check cycle failed, will retry next interval"
            log_info "Next check in ${CHECK_INTERVAL} seconds..."
            sleep "${CHECK_INTERVAL}"
        done
    else
        run_check_and_extend
    fi
}

main "$@"
