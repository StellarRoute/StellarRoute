#!/bin/bash
# StellarRoute Deployment Toolchain — Shared Helpers
# Source this file from other scripts: source "$(dirname "$0")/lib/common.sh"

set -euo pipefail

# ── Globals ───────────────────────────────────────────────────────────

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CONFIG_DIR="${REPO_ROOT}/config"
CONTRACTS_DIR="${REPO_ROOT}/crates/contracts"
WASM_TARGET_DIR="${CONTRACTS_DIR}/target/wasm32-unknown-unknown/release"
WASM_FILE="${WASM_TARGET_DIR}/stellarroute_contracts.wasm"
LOG_DIR="${REPO_ROOT}/logs"
NETWORK=""
IDENTITY="deployer"
DRY_RUN=false
DEFAULT_NETWORK="${STELLAR_NETWORK:-testnet}"

# ── Logging ───────────────────────────────────────────────────────────

log_info()  { echo "[INFO]  $(date -u +%Y-%m-%dT%H:%M:%SZ) $*"; }
log_ok()    { echo "[OK]    $(date -u +%Y-%m-%dT%H:%M:%SZ) $*"; }
log_warn()  { echo "[WARN]  $(date -u +%Y-%m-%dT%H:%M:%SZ) $*"; }
log_error() { echo "[ERROR] $(date -u +%Y-%m-%dT%H:%M:%SZ) $*" >&2; }
log_tx()    { echo "[TX]    $(date -u +%Y-%m-%dT%H:%M:%SZ) hash=$1 action=$2" | tee -a "${LOG_DIR}/${NETWORK}-tx.log"; }

# ── Network Resolution ────────────────────────────────────────────────

parse_network_flag() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --network)
                NETWORK="$2"
                shift 2
                ;;
            --identity)
                IDENTITY="$2"
                shift 2
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            *)
                shift
                ;;
        esac
    done

    if [[ -z "${NETWORK}" ]]; then
        NETWORK="${DEFAULT_NETWORK}"
    fi

    if [[ "${NETWORK}" != "testnet" && "${NETWORK}" != "mainnet" ]]; then
        log_error "Invalid network '${NETWORK}'. Must be 'testnet' or 'mainnet'."
        exit 1
    fi
}

get_rpc_url() {
    jq -r ".${NETWORK}.rpc_url" "${CONFIG_DIR}/networks.json"
}

get_network_passphrase() {
    jq -r ".${NETWORK}.network_passphrase" "${CONFIG_DIR}/networks.json"
}

# ── Deployment Artifact ───────────────────────────────────────────────

deployment_file() {
    echo "${CONFIG_DIR}/deployment-${NETWORK}.json"
}

# Committed, reviewable, non-secret artifact. Contains contract IDs and public
# metadata only — never keys, identities, or local filesystem paths.
public_deployment_file() {
    echo "${CONFIG_DIR}/deployments/${NETWORK}.json"
}

get_contract_id() {
    get_named_contract_id "router"
}

get_named_contract_id() {
    local contract_name="$1"
    local file
    file="$(deployment_file)"
    if [[ ! -f "${file}" ]]; then
        log_error "No deployment artifact found at ${file}. Run deploy.sh first."
        exit 1
    fi
    local id
    id="$(jq -r ".contracts.${contract_name}.contract_id // .contract_id // empty" "${file}")"
    if [[ -z "${id}" || "${id}" == "null" ]]; then
        log_error "No deployment record for '${contract_name}' found in ${file}."
        exit 1
    fi
    echo "${id}"
}

save_deployment() {
    local contract_json="$1"
    local file
    file="$(deployment_file)"
    cat > "${file}" <<ARTIFACT
{
  "contract_id": "$(echo "${contract_json}" | jq -r '.router.contract_id // empty')",
  "contracts": ${contract_json},
  "network": "${NETWORK}",
  "rpc_url": "$(get_rpc_url)",
  "deployed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "git_commit": "$(git -C "${REPO_ROOT}" rev-parse HEAD 2>/dev/null || echo 'unknown')"
}
ARTIFACT
    log_ok "Deployment artifact saved to ${file}"
}

# Write the committed, non-secret deploy artifact consumed by operators and by
# ROUTER_CONTRACT_ADDRESS. Fields here are all public: contract IDs, the public
# RPC URL, a UTC timestamp, and the git SHA that produced the build.
save_public_deployment() {
    local router_id="$1"
    local adapter_id="${2:-}"
    local file
    file="$(public_deployment_file)"
    mkdir -p "$(dirname "${file}")"

    cat > "${file}" <<ARTIFACT
{
  "network": "${NETWORK}",
  "router_contract_id": "${router_id}",
  "constant_product_adapter_contract_id": "${adapter_id}",
  "deployed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "git_sha": "$(git -C "${REPO_ROOT}" rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "rpc_url": "$(get_rpc_url)"
}
ARTIFACT

    log_ok "Public deployment artifact saved to ${file}"
}

# Post-deploy smoke call: prove the ID actually resolves on-chain before the
# artifact is trusted. Fails (non-zero) if the contract does not answer.
smoke_check_router() {
    local router_id="$1"
    log_info "Smoke-checking router ${router_id} via get_admin()..."
    if ! soroban_cmd contract invoke \
        --id "${router_id}" \
        --source "${IDENTITY}" \
        --network "${NETWORK}" \
        -- get_admin; then
        log_error "Smoke check FAILED: router ${router_id} did not respond to get_admin on ${NETWORK}"
        return 1
    fi
    log_ok "Smoke check passed for ${router_id}"
}

# ── Soroban Helpers ───────────────────────────────────────────────────

ensure_soroban_cli() {
    if ! command -v soroban &>/dev/null && ! command -v stellar &>/dev/null; then
        log_error "Soroban CLI (soroban or stellar) is not installed."
        log_error "Install: cargo install --locked soroban-cli"
        exit 1
    fi
}

soroban_cmd() {
    if command -v stellar &>/dev/null; then
        stellar "$@"
    else
        soroban "$@"
    fi
}

run_cmd() {
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY-RUN] $*"
        return 0
    fi
    "$@"
}

configure_network() {
    log_info "Configuring network: ${NETWORK}"
    run_cmd soroban_cmd network add "${NETWORK}" \
        --rpc-url "$(get_rpc_url)" \
        --network-passphrase "$(get_network_passphrase)" 2>/dev/null || true
}

invoke_contract() {
    local contract_id="$1"
    local fn_name="$2"
    shift 2
    soroban_cmd contract invoke \
        --id "${contract_id}" \
        --source "${IDENTITY}" \
        --network "${NETWORK}" \
        -- "${fn_name}" "$@"
}

# ── Build Helpers ─────────────────────────────────────────────────────

build_wasm() {
    log_info "Building contracts to WASM..."
    cargo build --manifest-path "${CONTRACTS_DIR}/Cargo.toml" \
        --target wasm32-unknown-unknown --release
    log_ok "WASM build complete: ${WASM_FILE}"
}

trap_with_context() {
    local line="$1"
    local exit_code="$2"
    log_error "Command failed (line ${line}) with exit code ${exit_code}"
    exit "${exit_code}"
}

optimize_wasm() {
    log_info "Optimizing WASM..."
    soroban_cmd contract optimize --wasm "${WASM_FILE}"
    log_ok "WASM optimized"
}

local_wasm_hash() {
    sha256sum "${WASM_FILE}" | awk '{print $1}'
}

# ── Init ──────────────────────────────────────────────────────────────

ensure_log_dir() {
    mkdir -p "${LOG_DIR}"
}
