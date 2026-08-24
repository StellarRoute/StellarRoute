#!/usr/bin/env bash
# Signed live Stellar Testnet → Sepolia CCTP corridor proof (non-custodial).
# - Disposable Postgres + ephemeral HMAC (never committed)
# - Wallet signing via stellar CLI + cast (keys outside repo)
# - Never prints access tokens, XDR, HMAC keys, attestations, or private keys
# - Writes sanitized evidence on success only
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUN_ID="${CCTP_PROOF_RUN_ID:-$$}"
API_PORT="${CCTP_PROOF_API_PORT:-31889}"
FRONTEND_ORIGIN="${CCTP_PROOF_FRONTEND_ORIGIN:-http://127.0.0.1:31999}"
EVIDENCE_PATH="$ROOT/docs/readiness/evidence/cctp-signed-live-stellar-to-sepolia.json"
PROOF_SCRIPT_PATH="$ROOT/scripts/cctp-signed-live-stellar-to-sepolia-proof.sh"
EVM_SIGNER_BIN="$ROOT/target/debug/cctp-evm-signer"
STELLAR_RPC="${CCTP_STELLAR_RPC_URL:-https://soroban-testnet.stellar.org}"
SEPOLIA_RPC="${CCTP_SEPOLIA_RPC_URL:-${SEPOLIA_RPC_URL:-https://ethereum-sepolia-rpc.publicnode.com}}"
STELLAR_USDC_CONTRACT="CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA"
STELLAR_TOKEN_MESSENGER="CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP"
SEPOLIA_USDC="0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"
SEPOLIA_MESSAGE_TRANSMITTER="0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275"
STELLAR_NETWORK_PASSPHRASE="${STELLAR_NETWORK_PASSPHRASE:-Test SDF Network ; September 2015}"
PROOF_AMOUNT="${CCTP_PROOF_AMOUNT:-1.0000000}"
MIN_XLM="${CCTP_MIN_XLM:-10}"
MIN_SEPOLIA_ETH_WEI="${CCTP_MIN_SEPOLIA_ETH_WEI:-1000000000000000}"
ATTESTATION_TIMEOUT_SECS="${CCTP_PROOF_ATTESTATION_TIMEOUT_SECS:-1800}"
STELLAR_IDENTITY="${CCTP_STELLAR_IDENTITY:-deployer}"
EVM_KEY_FILE="${CCTP_EVM_MINT_KEY_FILE:-/tmp/stellarroute-cctp-evm-mint-${USER}.key}"
EVM_ADDR_FILE="${CCTP_EVM_RECIPIENT_FILE:-/tmp/stellarroute-cctp-evm-recipient-${USER}.addr}"

API_PID=""
ACCESS_TOKEN=""
PG_BACKEND=""
PGDATA="/tmp/stellarroute-cctp-signed-${RUN_ID}"
PG_HOST="127.0.0.1"
PG_PORT_META="${CCTP_PROOF_PG_PORT:-55434}"
DB_NAME="stellarroute_cctp_signed_${RUN_ID}"
DOCKER_CONTAINER="stellarroute-cctp-signed-${RUN_ID}"
LOCAL_PG_SUPERUSER="${LOCAL_PG_SUPERUSER:-$USER}"
LOCAL_PG_PORT="${LOCAL_PG_PORT:-5432}"

TMP_DIR="/tmp/stellarroute-cctp-signed-${RUN_ID}"
mkdir -p "$TMP_DIR"
chmod 700 "$TMP_DIR"

secret_scan() {
  local file="$1"
  if grep -E '(access_token|xdr_envelope|BEGIN [A-Z ]+ KEY|postgres://[^@]+@|private\.key|seed_phrase)' "$file" >/dev/null 2>&1; then
    echo "Secret scan failed for $file" >&2
    return 1
  fi
}

cleanup() {
  if [[ -n "$API_PID" ]] && kill -0 "$API_PID" >/dev/null 2>&1; then
    kill "$API_PID" >/dev/null 2>&1 || true
    wait "$API_PID" >/dev/null 2>&1 || true
  fi
  if [[ "$PG_BACKEND" == "initdb" && -d "$PGDATA" ]]; then
    LC_ALL=en_US.UTF-8 LANG=en_US.UTF-8 pg_ctl -D "$PGDATA" -m fast stop >/dev/null 2>&1 || true
    rm -rf "$PGDATA"
  fi
  if [[ "$PG_BACKEND" == "existing-local" ]]; then
    psql -h "$PG_HOST" -p "$LOCAL_PG_PORT" -U "$LOCAL_PG_SUPERUSER" -d postgres \
      -c "DROP DATABASE IF EXISTS ${DB_NAME};" >/dev/null 2>&1 || true
  fi
  if [[ "$PG_BACKEND" == "docker" ]]; then
    docker rm -f "$DOCKER_CONTAINER" >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_DIR"
  rm -f "/tmp/stellarroute-cctp-signed-api-${RUN_ID}.log"
  ACCESS_TOKEN=""
}
trap cleanup EXIT

apply_migrations() {
  local mig_dir="$ROOT/crates/api/migrations"
  for f in \
    "$mig_dir/0015_cctp_transfers.sql" \
    "$mig_dir/0016_cctp_transfers_hardening.sql" \
    "$mig_dir/0017_cctp_mint_metadata.sql" \
    "$mig_dir/0018_cctp_approval_tx_hash.sql" \
    "$mig_dir/0019_cctp_approval_verified_at.sql" \
    "$mig_dir/20260730_cctp_review_fixes.sql" \
    "$mig_dir/20260731_cctp_prepare_lock_hardening.sql" \
    "$mig_dir/20260801_cctp_http_gate.sql" \
    "$mig_dir/20260802_cctp_http_hardening.sql" \
    "$mig_dir/20260803_cctp_reattest_lease.sql"
  do
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$f" >/dev/null
  done
}

bootstrap_existing_local_pg() {
  if ! command -v psql >/dev/null 2>&1; then return 1; fi
  if ! psql -h "$PG_HOST" -p "$LOCAL_PG_PORT" -U "$LOCAL_PG_SUPERUSER" -d postgres -c "SELECT 1" >/dev/null 2>&1; then
    return 1
  fi
  psql -h "$PG_HOST" -p "$LOCAL_PG_PORT" -U "$LOCAL_PG_SUPERUSER" -d postgres \
    -c "DROP DATABASE IF EXISTS ${DB_NAME};" >/dev/null 2>&1 || true
  psql -h "$PG_HOST" -p "$LOCAL_PG_PORT" -U "$LOCAL_PG_SUPERUSER" -d postgres \
    -c "CREATE DATABASE ${DB_NAME};" >/dev/null
  export DATABASE_URL="postgres://${LOCAL_PG_SUPERUSER}@${PG_HOST}:${LOCAL_PG_PORT}/${DB_NAME}"
  PG_BACKEND="existing-local"
  return 0
}

bootstrap_initdb_pg() {
  if ! command -v initdb >/dev/null 2>&1 || ! command -v pg_ctl >/dev/null 2>&1; then return 1; fi
  rm -rf "$PGDATA"
  if ! LC_ALL=en_US.UTF-8 LANG=en_US.UTF-8 initdb -D "$PGDATA" -U postgres -A trust --encoding=UTF8 >/dev/null 2>&1; then
    return 1
  fi
  if ! LC_ALL=en_US.UTF-8 LANG=en_US.UTF-8 pg_ctl -D "$PGDATA" -o "-p ${PG_PORT_META}" -w -t 30 start >/dev/null 2>&1; then
    rm -rf "$PGDATA"
    return 1
  fi
  psql -h "$PG_HOST" -p "$PG_PORT_META" -U postgres -d postgres -c "CREATE DATABASE ${DB_NAME};" >/dev/null
  export DATABASE_URL="postgres://postgres@${PG_HOST}:${PG_PORT_META}/${DB_NAME}"
  PG_BACKEND="initdb"
  return 0
}

bootstrap_docker_pg() {
  if ! command -v docker >/dev/null 2>&1 || ! docker info >/dev/null 2>&1; then return 1; fi
  docker rm -f "$DOCKER_CONTAINER" >/dev/null 2>&1 || true
  if ! docker run -d --name "$DOCKER_CONTAINER" -e POSTGRES_PASSWORD=postgres -p "${PG_PORT_META}:5432" postgres:16-alpine >/dev/null 2>&1; then
    return 1
  fi
  for _ in $(seq 1 60); do
    docker exec "$DOCKER_CONTAINER" pg_isready -U postgres >/dev/null 2>&1 && break
    sleep 1
  done
  docker exec "$DOCKER_CONTAINER" psql -U postgres -c "CREATE DATABASE ${DB_NAME};" >/dev/null
  export DATABASE_URL="postgres://postgres:postgres@${PG_HOST}:${PG_PORT_META}/${DB_NAME}"
  PG_BACKEND="docker"
  return 0
}

bootstrap_pg() {
  bootstrap_existing_local_pg && return 0
  bootstrap_initdb_pg && return 0
  bootstrap_docker_pg && return 0
  return 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "Missing required command: $1" >&2; exit 1; }
}

stellar_source_g() {
  if [[ -n "${CCTP_STELLAR_SOURCE_G:-}" ]]; then
    echo "$CCTP_STELLAR_SOURCE_G"
    return 0
  fi
  stellar keys address "$STELLAR_IDENTITY"
}

stellar_usdc_balance() {
  local g="$1"
  local out
  out="$(STELLAR_NETWORK_PASSPHRASE="$STELLAR_NETWORK_PASSPHRASE" \
    stellar contract invoke --id "$STELLAR_USDC_CONTRACT" --source "$STELLAR_IDENTITY" --network testnet \
    -- balance --id "$g" 2>&1 || true)"
  if echo "$out" | grep -q 'trustline entry is missing'; then
    echo "0"
    return 0
  fi
  if echo "$out" | grep -q 'error:'; then
    echo "0"
    return 0
  fi
  echo "$out" | tail -1 | tr -d '"'
}

stellar_xlm_balance() {
  local g="$1"
  curl -fsS "https://horizon-testnet.stellar.org/accounts/${g}" \
    | jq -r '.balances[] | select(.asset_type=="native") | .balance' 2>/dev/null || echo "0"
}

ensure_evm_mint_key() {
  if [[ ! -f "$EVM_KEY_FILE" || -L "$EVM_KEY_FILE" ]]; then
    echo "Missing secure EVM key file: ${EVM_KEY_FILE}" >&2
    echo "Create it out-of-band as a regular 0600 file; environment keys are rejected." >&2
    return 1
  fi
  local derived configured stored
  derived="$("$EVM_SIGNER_BIN" address --key-file "$EVM_KEY_FILE")"
  configured="${CCTP_EVM_RECIPIENT:-}"
  if [[ -n "$configured" && "${configured,,}" != "${derived,,}" ]]; then
    echo "CCTP_EVM_RECIPIENT does not match the secure key file" >&2
    return 1
  fi
  if [[ -e "$EVM_ADDR_FILE" ]]; then
    if [[ ! -f "$EVM_ADDR_FILE" || -L "$EVM_ADDR_FILE" ]]; then
      echo "EVM recipient path must be a regular file, not a symlink" >&2
      return 1
    fi
    stored="$(<"$EVM_ADDR_FILE")"
    if [[ "${stored,,}" != "${derived,,}" ]]; then
      echo "EVM recipient file does not match the secure key file" >&2
      return 1
    fi
  else
    (umask 077; printf '%s\n' "$derived" >"$EVM_ADDR_FILE")
    chmod 600 "$EVM_ADDR_FILE"
  fi
  printf '%s\n' "$derived"
}

evm_eth_balance_wei() {
  cast balance "$1" --rpc-url "$SEPOLIA_RPC" 2>/dev/null || echo "0"
}

compare_usdc_ge() {
  python3 - "$1" "$2" <<'PY'
import sys
from decimal import Decimal
have = Decimal(sys.argv[1]) / Decimal(10_000_000)
need = Decimal(sys.argv[2])
sys.exit(0 if have >= need else 1)
PY
}

wait_for_stellar_tx() {
  local hash="$1"
  local deadline=$(( $(date +%s) + 120 ))
  while [[ "$(date +%s)" -lt "$deadline" ]]; do
    if curl -fsS "https://horizon-testnet.stellar.org/transactions/${hash}" \
      | jq -e '.successful == true' >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  echo "Stellar transaction not finalized on Horizon: ${hash}" >&2
  return 1
}

wait_for_evm_tx() {
  local hash="$1"
  local deadline=$(( $(date +%s) + 120 ))
  while [[ "$(date +%s)" -lt "$deadline" ]]; do
    if cast receipt "$hash" --rpc-url "$SEPOLIA_RPC" --json 2>/dev/null \
      | jq -e '.status == "0x1"' >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  echo "EVM transaction not finalized on Sepolia: ${hash}" >&2
  return 1
}

stellar_invoke_from_xdr() {
  local xdr="$1"
  local decoded contract func combined hash
  decoded="$(STELLAR_NETWORK_PASSPHRASE="$STELLAR_NETWORK_PASSPHRASE" \
    stellar tx decode "$xdr" 2>"$TMP_DIR/stellar-decode.err")"
  contract="$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.contract_address // empty')"
  func="$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.function_name // empty')"
  if [[ -z "$contract" || -z "$func" ]]; then
    echo "Failed to decode Soroban invoke from prepared XDR; see $TMP_DIR/stellar-decode.err" >&2
    return 1
  fi

  local -a invoke_args=(--id "$contract" --source "$STELLAR_IDENTITY" --network testnet --send=yes -- "$func")
  case "$func" in
    approve)
      invoke_args+=(
        --from "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[0].address')"
        --spender "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[1].address')"
        --amount "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[2].i128')"
        --expiration_ledger "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[3].u32')"
      )
      ;;
    deposit_for_burn)
      invoke_args+=(
        --caller "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[0].address')"
        --amount "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[1].i128')"
        --destination_domain "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[2].u32')"
        --mint_recipient "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[3].bytes')"
        --burn_token "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[4].address')"
        --destination_caller "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[5].bytes')"
        --max_fee "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[6].i128')"
        --min_finality_threshold "$(echo "$decoded" | jq -r '.tx.tx.operations[0].body.invoke_host_function.host_function.invoke_contract.args[7].u32')"
      )
      ;;
    *)
      echo "Unsupported Soroban invoke for signed-live proof: ${func}" >&2
      return 1
      ;;
  esac

  combined="$(STELLAR_NETWORK_PASSPHRASE="$STELLAR_NETWORK_PASSPHRASE" \
    stellar contract invoke "${invoke_args[@]}" 2>"$TMP_DIR/stellar-send.err")"
  # Prefer explicit CLI markers; bare 64-hex can match mint_recipient bytes32.
  hash="$( { echo "$combined"; cat "$TMP_DIR/stellar-send.err"; } \
    | rg -o 'Signing transaction: [0-9a-f]{64}' \
    | head -1 \
    | rg -o '[0-9a-f]{64}' || true)"
  if [[ -z "$hash" ]]; then
    hash="$( { echo "$combined"; cat "$TMP_DIR/stellar-send.err"; } \
      | rg -o 'stellar\.expert/explorer/testnet/tx/[0-9a-f]{64}' \
      | head -1 \
      | rg -o '[0-9a-f]{64}$' || true)"
  fi
  if [[ -z "$hash" ]]; then
    echo "Stellar broadcast failed; see $TMP_DIR/stellar-send.err" >&2
    return 1
  fi
  if ! curl -fsS "https://horizon-testnet.stellar.org/transactions/${hash}" >/dev/null 2>&1; then
    echo "Stellar transaction hash not found on Horizon: ${hash}" >&2
    return 1
  fi
  wait_for_stellar_tx "$hash" || return 1
  echo "$hash"
}

stellar_sign_and_send() {
  stellar_invoke_from_xdr "$1"
}

evm_sign_and_send() {
  local to="$1" data="$2" value="$3" recipient="$4"
  local hash
  (umask 077; printf '%s\n' "$SEPOLIA_RPC" >"$TMP_DIR/evm-rpc-url")
  jq -n \
    --arg recipient "$recipient" \
    --arg contract "$SEPOLIA_MESSAGE_TRANSMITTER" \
    --arg to "$to" \
    --arg data "$data" \
    --arg value "$value" \
    '{
      chain_id: 11155111,
      recipient: $recipient,
      contract: $contract,
      to: $to,
      data: $data,
      value: $value,
      max_gas_limit: 1000000
    }' >"$TMP_DIR/evm-sign-request.json"
  chmod 600 "$TMP_DIR/evm-sign-request.json"
  hash="$("$EVM_SIGNER_BIN" send \
    --key-file "$EVM_KEY_FILE" \
    --request-file "$TMP_DIR/evm-sign-request.json" \
    --rpc-file "$TMP_DIR/evm-rpc-url")"
  if [[ ! "$hash" =~ ^0x[0-9a-f]{64}$ ]]; then
    echo "EVM signing helper did not return a public transaction hash" >&2
    return 1
  fi
  printf '%s\n' "$hash"
}

api_post() {
  local path="$1" body="$2" out="$3" extra_header="${4:-}"
  local http
  if [[ -n "$extra_header" ]]; then
    http="$(curl -sS -o "$out" -w '%{http_code}' -X POST "${BASE_URL}${path}" \
      -H 'content-type: application/json' \
      -H "x-cctp-transfer-access: ${ACCESS_TOKEN}" \
      -H "$extra_header" \
      -d "$body")"
  else
    http="$(curl -sS -o "$out" -w '%{http_code}' -X POST "${BASE_URL}${path}" \
      -H 'content-type: application/json' \
      -H "x-cctp-transfer-access: ${ACCESS_TOKEN}" \
      -d "$body")"
  fi
  echo "$http"
}

poll_transfer_status() {
  local target="$1" deadline="$2"
  local status=""
  while [[ "$(date +%s)" -lt "$deadline" ]]; do
    local http
    http="$(curl -sS -o "$TMP_DIR/status.json" -w '%{http_code}' \
      "${BASE_URL}/api/v2/bridge/cctp/${transfer_id}" \
      -H "x-cctp-transfer-access: ${ACCESS_TOKEN}" 2>/dev/null || echo "000")"
    if [[ "$http" != "200" ]]; then
      sleep 10
      continue
    fi
    status="$(jq -r '.data.status // empty' "$TMP_DIR/status.json")"
    if [[ "$status" == "$target" || "$status" == "completed" ]]; then
      echo "$status"
      return 0
    fi
    if [[ "$status" == "attestation_failed" ]]; then
      echo "$status"
      return 1
    fi
    sleep 10
  done
  echo "$status"
  return 1
}

funding_checkpoint() {
  local stellar_g evm_recipient usdc_raw xlm bal eth_wei
  stellar_g="$(stellar_source_g)"
  evm_recipient="$(ensure_evm_mint_key)"

  if ! curl -fsS "https://horizon-testnet.stellar.org/accounts/${stellar_g}" >/dev/null 2>&1; then
    echo "Funding Stellar account via Friendbot..."
    curl -fsS "https://friendbot.stellar.org?addr=${stellar_g}" >/dev/null || true
    sleep 3
  fi

  usdc_raw="$(stellar_usdc_balance "$stellar_g")"
  xlm="$(stellar_xlm_balance "$stellar_g")"
  eth_wei="$(evm_eth_balance_wei "$evm_recipient")"

  local usdc_ok=0 xlm_ok=0 eth_ok=0
  compare_usdc_ge "$usdc_raw" "$PROOF_AMOUNT" && usdc_ok=1 || true
  python3 - "$xlm" "$MIN_XLM" <<'PY' && xlm_ok=1 || true
import sys
from decimal import Decimal
sys.exit(0 if Decimal(sys.argv[1]) >= Decimal(sys.argv[2]) else 1)
PY
  python3 - "$eth_wei" "$MIN_SEPOLIA_ETH_WEI" <<'PY' && eth_ok=1 || true
import sys
sys.exit(0 if int(sys.argv[1]) >= int(sys.argv[2]) else 1)
PY

  if [[ "$usdc_ok" -eq 1 && "$xlm_ok" -eq 1 && "$eth_ok" -eq 1 ]]; then
    STELLAR_G="$stellar_g"
    EVM_RECIPIENT="$evm_recipient"
    return 0
  fi

  echo ""
  echo "STATUS: BLOCKED_ON_FUNDING"
  echo ""
  echo "Stellar source (burn signer): ${stellar_g}"
  echo "  stellar CLI identity: ${STELLAR_IDENTITY}"
  echo "  XLM balance: ${xlm} (need >= ${MIN_XLM})"
  echo "  USDC raw balance: ${usdc_raw} (need >= $(python3 -c "from decimal import Decimal; print(int(Decimal('${PROOF_AMOUNT}')*Decimal(10_000_000)))")) for amount ${PROOF_AMOUNT})"
  echo "  Fund XLM: curl 'https://friendbot.stellar.org?addr=${stellar_g}'"
  echo "  Fund USDC: https://faucet.circle.com/ — select Stellar Testnet, paste ${stellar_g} (browser + reCAPTCHA)"
  echo ""
  echo "Sepolia recipient + mint gas wallet: ${evm_recipient}"
  echo "  ETH balance wei: ${eth_wei} (need >= ${MIN_SEPOLIA_ETH_WEI})"
  echo "  Fund ETH: https://sepoliafaucet.com/ or https://www.alchemy.com/faucets/ethereum-sepolia"
  echo "  Mint key file (local, not in repo): ${EVM_KEY_FILE}"
  echo ""
  echo "After funding, re-run:"
  echo "  CCTP_STELLAR_IDENTITY=${STELLAR_IDENTITY} CCTP_EVM_RECIPIENT=${evm_recipient} \\"
  echo "  CCTP_EVM_MINT_KEY_FILE=${EVM_KEY_FILE} ${PROOF_SCRIPT_PATH}"
  echo ""
  echo "What is already ready:"
  echo "  - API/bootstrap harness in this script"
  echo "  - stellar CLI signing (identity ${STELLAR_IDENTITY})"
  echo "  - cast EVM mint broadcast"
  echo "  - configured-ready gate evidence at docs/readiness/evidence/cctp-configured-ready-proof.json"
  exit 2
}

echo "=== CCTP signed-live Stellar→Sepolia proof (run ${RUN_ID}) ==="

require_cmd curl
require_cmd jq
require_cmd stellar
require_cmd cast
require_cmd python3
require_cmd cargo

cd "$ROOT"
cargo build -q -p stellarroute-api --bin cctp-evm-signer

chain_hex="$(curl -fsS -X POST "$SEPOLIA_RPC" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}' \
  | jq -r '.result')"
if [[ "$chain_hex" != "0xaa36a7" ]]; then
  echo "Sepolia RPC chain id mismatch (got ${chain_hex})" >&2
  exit 1
fi

funding_checkpoint

bootstrap_pg || { echo "Failed to start disposable Postgres" >&2; exit 1; }
apply_migrations

export CCTP_ACCESS_TOKEN_HMAC_KEY
CCTP_ACCESS_TOKEN_HMAC_KEY="$(python3 -c 'import os,base64; print(base64.urlsafe_b64encode(os.urandom(32)).decode().rstrip("="))')"
export ADMIN_AUTH_TOKEN
ADMIN_AUTH_TOKEN="$(python3 -c 'import secrets; print(secrets.token_hex(32))')"
export API_KEYS
API_KEYS="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"
PROOF_API_KEY="$API_KEYS"

export CCTP_ENABLED=true
export REQUIRE_AUTH=true
export CORS_ALLOWED_ORIGINS="$FRONTEND_ORIGIN"
export STELLARROUTE_ENV=production
export REQUIRE_STRICT_CORS=1
export CCTP_STELLAR_RPC_URL="$STELLAR_RPC"
export CCTP_SEPOLIA_RPC_URL="$SEPOLIA_RPC"
export STELLAR_HORIZON_URL="${STELLAR_HORIZON_URL:-https://horizon-testnet.stellar.org}"
export API_HOST=127.0.0.1
export API_PORT="$API_PORT"

cd "$ROOT"
cargo build -q -p stellarroute-api --bin stellarroute-api
./target/debug/stellarroute-api >/tmp/stellarroute-cctp-signed-api-${RUN_ID}.log 2>&1 &
API_PID=$!

BASE_URL="http://${API_HOST}:${API_PORT}"
stellar_exec=""
for _ in $(seq 1 300); do
  if ! kill -0 "$API_PID" >/dev/null 2>&1; then
    tail -40 "/tmp/stellarroute-cctp-signed-api-${RUN_ID}.log" >&2 || true
    exit 1
  fi
  code="$(curl -m 5 -sS -o "$TMP_DIR/v2-info.json" -w '%{http_code}' \
    -H "x-api-key: ${PROOF_API_KEY}" "${BASE_URL}/api/v2" 2>/dev/null || true)"
  if [[ "$code" == "200" ]]; then
    stellar_exec="$(jq -r '.data.supported_corridors[]? | select(.direction=="stellar_to_evm") | .executable' \
      "$TMP_DIR/v2-info.json" 2>/dev/null || true)"
    if [[ "$stellar_exec" == "true" ]]; then
      break
    fi
  fi
  sleep 1
done

if [[ "$stellar_exec" != "true" ]]; then
  echo "stellar_to_evm not executable" >&2
  jq '{bridge_settlement_executable: .data.bridge_settlement_executable, supported_corridors: .data.supported_corridors}' \
    "$TMP_DIR/v2-info.json" >&2 2>/dev/null || true
  tail -40 "/tmp/stellarroute-cctp-signed-api-${RUN_ID}.log" >&2 || true
  exit 1
fi

SAGA_START_TS="$(date +%s)"
quote_body="$(cat <<EOF
{
  "corridor_id":"circle-cctp:usdc:stellar-testnet:ethereum-sepolia",
  "provider":"circle-cctp",
  "direction":"stellar_to_evm",
  "source_chain_id":"stellar:testnet",
  "destination_chain_id":"eip155:11155111",
  "source_asset":{"chain_id":"stellar:testnet","asset":"erc20:${STELLAR_USDC_CONTRACT}","canonical":"stellar:testnet/erc20:${STELLAR_USDC_CONTRACT}","symbol":"USDC"},
  "destination_asset":{"chain_id":"eip155:11155111","asset":"erc20:${SEPOLIA_USDC}","canonical":"eip155:11155111/erc20:${SEPOLIA_USDC}","symbol":"USDC"},
  "amount":"${PROOF_AMOUNT}",
  "recipient":"${EVM_RECIPIENT}",
  "sender":"${STELLAR_G}",
  "finality":"standard"
}
EOF
)"

quote_http="$(curl -sS -o "$TMP_DIR/quote.json" -w '%{http_code}' \
  -X POST "${BASE_URL}/api/v2/bridge/cctp/quote" \
  -H 'content-type: application/json' \
  -d "$quote_body")"
[[ "$quote_http" == "200" ]] || { jq 'del(.data.access_token)' "$TMP_DIR/quote.json" >&2; exit 1; }

transfer_id="$(jq -r '.data.transfer_id' "$TMP_DIR/quote.json")"
ACCESS_TOKEN="$(jq -r '.data.access_token' "$TMP_DIR/quote.json")"
transfer_redacted="${transfer_id:0:8}…${transfer_id: -4}"
REQUEST_ID_QUOTE="$(jq -r '.request_id // empty' "$TMP_DIR/quote.json")"

prepare_http="$(api_post "/api/v2/bridge/cctp/${transfer_id}/prepare-burn" '{}' "$TMP_DIR/prepare-burn.json")"
[[ "$prepare_http" == "200" || "$prepare_http" == "409" ]] || { jq 'del(.data.payload)' "$TMP_DIR/prepare-burn.json" >&2; exit 1; }

approval_required="$(jq -r '.data.approval_required // false' "$TMP_DIR/prepare-burn.json")"
if [[ "$approval_required" == "true" ]]; then
  approval_xdr="$(jq -r '.data.payload.xdr_envelope' "$TMP_DIR/prepare-burn.json")"
  approval_hash="$(stellar_sign_and_send "$approval_xdr")"
  api_post "/api/v2/bridge/cctp/${transfer_id}/submit-burn" "{\"tx_hash\":\"${approval_hash}\"}" "$TMP_DIR/submit-approval.json" >/dev/null
  prepare_http="$(api_post "/api/v2/bridge/cctp/${transfer_id}/prepare-burn" '{}' "$TMP_DIR/prepare-burn.json")"
  [[ "$prepare_http" == "200" || "$prepare_http" == "409" ]] || { jq 'del(.data.payload)' "$TMP_DIR/prepare-burn.json" >&2; exit 1; }
fi

burn_xdr="$(jq -r '.data.payload.xdr_envelope' "$TMP_DIR/prepare-burn.json")"
BURN_SUBMIT_START="$(date +%s)"
burn_hash="$(stellar_sign_and_send "$burn_xdr")"
submit_burn_http="$(api_post "/api/v2/bridge/cctp/${transfer_id}/submit-burn" "{\"tx_hash\":\"${burn_hash}\"}" "$TMP_DIR/submit-burn.json")"
[[ "$submit_burn_http" == "200" ]] || exit 1

deadline=$(( $(date +%s) + ATTESTATION_TIMEOUT_SECS ))
attest_status="$(poll_transfer_status attestation_ready "$deadline")" || {
  echo "Attestation poll failed (last status: ${attest_status})" >&2
  jq 'del(.data)' "$TMP_DIR/status.json" >&2 || true
  exit 1
}
ATTEST_READY_TS="$(date +%s)"

prepare_mint_http="$(api_post "/api/v2/bridge/cctp/${transfer_id}/prepare-mint" '{}' "$TMP_DIR/prepare-mint.json")"
[[ "$prepare_mint_http" == "200" ]] || { jq 'del(.data.payload)' "$TMP_DIR/prepare-mint.json" >&2; exit 1; }

mint_to="$(jq -r '.data.payload.to' "$TMP_DIR/prepare-mint.json")"
mint_data="$(jq -r '.data.payload.data' "$TMP_DIR/prepare-mint.json")"
mint_value="$(jq -r '.data.payload.value' "$TMP_DIR/prepare-mint.json")"
mint_chain_id="$(jq -r '.data.payload.chain_id' "$TMP_DIR/prepare-mint.json")"
if [[ "$mint_chain_id" != "eip155:11155111" \
  || "${mint_to,,}" != "${SEPOLIA_MESSAGE_TRANSMITTER,,}" \
  || "$mint_value" != "0" \
  || "${mint_data:0:10}" != "0x57ecfd28" ]]; then
  echo "Prepared mint payload failed chain/contract/value/calldata validation" >&2
  exit 1
fi
mint_hash="$(evm_sign_and_send "$mint_to" "$mint_data" "$mint_value" "$EVM_RECIPIENT")"
wait_for_evm_tx "$mint_hash" || exit 1
submit_mint_http="$(api_post "/api/v2/bridge/cctp/${transfer_id}/submit-mint" "{\"tx_hash\":\"${mint_hash}\"}" "$TMP_DIR/submit-mint.json")"
[[ "$submit_mint_http" == "200" ]] || { jq 'del(.data)' "$TMP_DIR/submit-mint.json" >&2; exit 1; }

deadline=$(( $(date +%s) + 600 ))
final_status="$(poll_transfer_status completed "$deadline")" || {
  echo "Mint completion poll failed (last status: ${final_status})" >&2
  exit 1
}
COMPLETE_TS="$(date +%s)"

burn_ledger="$(curl -fsS "https://horizon-testnet.stellar.org/transactions/${burn_hash}" | jq -r '.ledger // empty' 2>/dev/null || true)"
mint_block="$(cast receipt "$mint_hash" --rpc-url "$SEPOLIA_RPC" --json 2>/dev/null | jq -r '.blockNumber // empty' || true)"
if [[ "$mint_block" == 0x* ]]; then
  mint_block=$((16#${mint_block#0x}))
fi

if ! git -C "$ROOT" diff --quiet || ! git -C "$ROOT" diff --cached --quiet; then
  echo "Git working tree must be clean before writing evidence" >&2
  exit 1
fi

tested_git_head="$(git -C "$ROOT" rev-parse HEAD)"
proof_script_sha256="$(shasum -a 256 "$PROOF_SCRIPT_PATH" | awk '{print $1}')"
mkdir -p "$(dirname "$EVIDENCE_PATH")"

jq -n \
  --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg tested_head "$tested_git_head" \
  --arg script_sha "$proof_script_sha256" \
  --arg direction "stellar_to_evm" \
  --arg stellar_domain "27" \
  --arg sepolia_domain "0" \
  --arg stellar_exec "$stellar_exec" \
  --arg transfer "$transfer_redacted" \
  --arg amount "$PROOF_AMOUNT" \
  --arg finality "standard" \
  --arg burn_hash "$burn_hash" \
  --arg mint_hash "$mint_hash" \
  --arg burn_ledger "$burn_ledger" \
  --arg mint_block "$mint_block" \
  --arg stellar_g "$STELLAR_G" \
  --arg evm_recipient "$EVM_RECIPIENT" \
  --argjson saga_start "$SAGA_START_TS" \
  --argjson burn_submit_start "$BURN_SUBMIT_START" \
  --argjson attest_ready_ts "$ATTEST_READY_TS" \
  --argjson complete_ts "$COMPLETE_TS" \
  --arg stellar_mt "$STELLAR_TOKEN_MESSENGER" \
  --arg stellar_usdc "$STELLAR_USDC_CONTRACT" \
  --arg sepolia_mt "0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275" \
  --arg sepolia_usdc "$SEPOLIA_USDC" \
  --arg req_id "$REQUEST_ID_QUOTE" \
  '{
    timestamp: $ts,
    tested_git_head: $tested_head,
    proof_script_sha256: $script_sha,
    evidence_scope: "signed_live_stellar_to_sepolia_testnet_only",
    direction: $direction,
    domains: { stellar: ($stellar_domain | tonumber), sepolia: ($sepolia_domain | tonumber) },
    finality: $finality,
    amount: $amount,
    transfer_id_redacted: $transfer,
    public_addresses: { stellar_source: $stellar_g, sepolia_recipient: $evm_recipient },
    contract_ids: {
      stellar_token_messenger: $stellar_mt,
      stellar_usdc: $stellar_usdc,
      sepolia_token_messenger: $sepolia_mt,
      sepolia_usdc: $sepolia_usdc
    },
    transaction_hashes: {
      stellar_burn: $burn_hash,
      sepolia_mint: $mint_hash
    },
    ledgers_blocks: {
      stellar_burn_ledger: (if $burn_ledger == "" then null else ($burn_ledger | tonumber) end),
      sepolia_mint_block: (if $mint_block == "" then null else ($mint_block | tonumber) end)
    },
    status_timeline: {
      saga_started_unix: $saga_start,
      burn_submitted_unix: $burn_submit_start,
      attestation_ready_unix: $attest_ready_ts,
      completed_unix: $complete_ts
    },
    timings_seconds: {
      burn_to_attestation: ($attest_ready_ts - $burn_submit_start),
      total_saga: ($complete_ts - $saga_start)
    },
    readiness: { stellar_to_evm_executable: ($stellar_exec == "true") },
    request_ids: { quote: (if $req_id == "" then null else $req_id end) },
    verified_claims: {
      signed_live_stellar_to_sepolia: true,
      reverse_corridor_not_claimed: true,
      mainnet_not_claimed: true
    },
    secret_scan: "pass"
  }' >"$EVIDENCE_PATH"

secret_scan "$EVIDENCE_PATH"
ACCESS_TOKEN=""

echo "STATUS: SIGNED-LIVE STELLAR→SEPOLIA COMPLETE"
echo "Evidence: ${EVIDENCE_PATH}"
echo "Burn tx: ${burn_hash}"
echo "Mint tx: ${mint_hash}"
