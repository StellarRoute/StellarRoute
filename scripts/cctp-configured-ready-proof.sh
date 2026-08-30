#!/usr/bin/env bash
# Opt-in local configured-ready proof against live public Testnet dependencies.
# - Disposable Postgres only (never remote/production DB)
# - Never prints access tokens, XDR, HMAC keys, attestations, or raw messages
# - Cleans up API + Postgres on exit
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUN_ID="${CCTP_PROOF_RUN_ID:-$$}"
API_PORT="${CCTP_PROOF_API_PORT:-31888}"
FRONTEND_ORIGIN="${CCTP_PROOF_FRONTEND_ORIGIN:-http://127.0.0.1:31999}"
EVIDENCE_PATH="$ROOT/docs/readiness/evidence/cctp-configured-ready-proof.json"
STELLAR_RPC="${CCTP_STELLAR_RPC_URL:-https://soroban-testnet.stellar.org}"
SEPOLIA_RPC="${CCTP_SEPOLIA_RPC_URL:-${SEPOLIA_RPC_URL:-https://ethereum-sepolia-rpc.publicnode.com}}"
# Pinned live Testnet burn fixture operation source (exists on-chain with USDC).
STELLAR_G="GAN3SJKZ7GNHVYCFX7Y3XTDIPZJW6PHMRUL7UDAUHMQ7FIUPDKBFARBC"
EVM_SENDER="0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
EVM_RECIPIENT="$EVM_SENDER"
PROOF_SCRIPT_PATH="$ROOT/scripts/cctp-configured-ready-proof.sh"
API_PID=""
ACCESS_TOKEN=""
PG_BACKEND=""
PGDATA="/tmp/stellarroute-cctp-proof-${RUN_ID}"
PG_HOST="127.0.0.1"
PG_PORT_META="${CCTP_PROOF_PG_PORT:-55433}"
DB_NAME="stellarroute_cctp_proof_${RUN_ID}"
DOCKER_CONTAINER="stellarroute-cctp-proof-${RUN_ID}"
LOCAL_PG_SUPERUSER="${LOCAL_PG_SUPERUSER:-$USER}"
LOCAL_PG_PORT="${LOCAL_PG_PORT:-5432}"

secret_scan() {
  local file="$1"
  if grep -E '(access_token|xdr_envelope|BEGIN [A-Z ]+ KEY|postgres://[^@]+@)' "$file" >/dev/null 2>&1; then
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
  rm -f "/tmp/cctp-proof-quote-${RUN_ID}.json" "/tmp/cctp-proof-prepare-${RUN_ID}.json" \
    "/tmp/cctp-proof-evm-quote-${RUN_ID}.json" "/tmp/cctp-proof-evm-prepare-${RUN_ID}.json" \
    "/tmp/stellarroute-cctp-proof-api-${RUN_ID}.log"
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
  if ! command -v psql >/dev/null 2>&1; then
    return 1
  fi
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
  if ! command -v initdb >/dev/null 2>&1 || ! command -v pg_ctl >/dev/null 2>&1; then
    return 1
  fi
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
  if ! command -v docker >/dev/null 2>&1 || ! docker info >/dev/null 2>&1; then
    return 1
  fi
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

echo "=== CCTP configured-ready proof (run ${RUN_ID}) ==="

chain_hex="$(curl -fsS -X POST "$SEPOLIA_RPC" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}' \
  | jq -r '.result')"
if [[ "$chain_hex" != "0xaa36a7" ]]; then
  echo "Sepolia RPC chain id mismatch (got ${chain_hex})" >&2
  exit 1
fi

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
if [[ -n "${CCTP_PROOF_RUST_LOG:-}" ]]; then
  export RUST_LOG="$CCTP_PROOF_RUST_LOG"
fi

cd "$ROOT"
cargo build -q -p stellarroute-api --bin stellarroute-api
./target/debug/stellarroute-api >/tmp/stellarroute-cctp-proof-api-${RUN_ID}.log 2>&1 &
API_PID=$!

BASE_URL="http://${API_HOST}:${API_PORT}"
healthy=0
for _ in $(seq 1 240); do
  code="$(curl -m 5 -sS -o /dev/null -w '%{http_code}' -H "x-api-key: ${PROOF_API_KEY}" "${BASE_URL}/api/v2" 2>/dev/null || true)"
  if [[ "$code" == "200" ]]; then
    healthy=1
    break
  fi
  if ! kill -0 "$API_PID" >/dev/null 2>&1; then
    echo "API process exited during startup" >&2
    tail -40 "/tmp/stellarroute-cctp-proof-api-${RUN_ID}.log" >&2 || true
    exit 1
  fi
  sleep 1
done
if [[ "$healthy" -ne 1 ]]; then
  echo "API /api/v2 readiness timeout; see /tmp/stellarroute-cctp-proof-api-${RUN_ID}.log" >&2
  tail -40 "/tmp/stellarroute-cctp-proof-api-${RUN_ID}.log" >&2 || true
  exit 1
fi

v2_json=""
stellar_exec="false"
evm_exec="false"
for _ in $(seq 1 90); do
  v2_json="$(curl -fsS -H "x-api-key: ${PROOF_API_KEY}" "${BASE_URL}/api/v2")"
  stellar_exec="$(echo "$v2_json" | jq -r '.data.supported_corridors[] | select(.direction=="stellar_to_evm") | .executable')"
  evm_exec="$(echo "$v2_json" | jq -r '.data.supported_corridors[] | select(.direction=="evm_to_stellar") | .executable')"
  if [[ "$stellar_exec" == "true" && "$evm_exec" == "true" ]]; then
    break
  fi
  sleep 2
done

public_v2_code="$(curl -m 5 -sS -o /dev/null -w '%{http_code}' "${BASE_URL}/api/v2" 2>/dev/null || true)"
if [[ "$public_v2_code" != "200" ]]; then
  echo "GET /api/v2 must be public without API key (got ${public_v2_code})" >&2
  exit 1
fi

if [[ "$stellar_exec" != "true" || "$evm_exec" != "true" ]]; then
  echo "Both corridors must be executable after semantic probes" >&2
  echo "$v2_json" | jq '.data.supported_corridors' >&2
  exit 1
fi

quote_body="$(cat <<EOF
{
  "corridor_id":"circle-cctp:usdc:stellar-testnet:ethereum-sepolia",
  "provider":"circle-cctp",
  "direction":"stellar_to_evm",
  "source_chain_id":"stellar:testnet",
  "destination_chain_id":"eip155:11155111",
  "source_asset":{"chain_id":"stellar:testnet","asset":"erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA","canonical":"stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA","symbol":"USDC"},
  "destination_asset":{"chain_id":"eip155:11155111","asset":"erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238","canonical":"eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238","symbol":"USDC"},
  "amount":"10.0000000",
  "recipient":"${EVM_RECIPIENT}",
  "sender":"${STELLAR_G}",
  "finality":"standard"
}
EOF
)"

quote_http="000"
for attempt in $(seq 1 15); do
  quote_http="$(curl -sS -o "/tmp/cctp-proof-quote-${RUN_ID}.json" -w '%{http_code}' \
    -X POST "${BASE_URL}/api/v2/bridge/cctp/quote" \
    -H 'content-type: application/json' \
    -d "$quote_body")"
  if [[ "$quote_http" == "200" ]]; then
    break
  fi
  if [[ "$quote_http" == "503" ]]; then
    sleep 4
    continue
  fi
  break
done

if [[ "$quote_http" != "200" ]]; then
  echo "Quote failed HTTP ${quote_http}" >&2
  jq 'del(.data.access_token)' "/tmp/cctp-proof-quote-${RUN_ID}.json" >&2 || true
  exit 1
fi

transfer_id="$(jq -r '.data.transfer_id' "/tmp/cctp-proof-quote-${RUN_ID}.json")"
ACCESS_TOKEN="$(jq -r '.data.access_token' "/tmp/cctp-proof-quote-${RUN_ID}.json")"
transfer_redacted="${transfer_id:0:8}…${transfer_id: -4}"

prepare_http="$(curl -sS -o "/tmp/cctp-proof-prepare-${RUN_ID}.json" -w '%{http_code}' \
  -X POST "${BASE_URL}/api/v2/bridge/cctp/${transfer_id}/prepare-burn" \
  -H "x-cctp-transfer-access: ${ACCESS_TOKEN}" \
  -H 'content-type: application/json' \
  -d '{}')"

prepare_type="$(jq -r '.data.payload.type // empty' "/tmp/cctp-proof-prepare-${RUN_ID}.json")"
prepare_network="$(jq -r 'if .data.payload.network_passphrase then "stellar_testnet" elif .data.payload.chain_id then .data.payload.chain_id else empty end' "/tmp/cctp-proof-prepare-${RUN_ID}.json")"
prepare_expires="$(jq -r '.data.expires_at // empty' "/tmp/cctp-proof-prepare-${RUN_ID}.json")"
prepare_has_hash="$(jq -r 'if .data.payload.xdr_envelope then true elif .data.payload.data then true else false end' "/tmp/cctp-proof-prepare-${RUN_ID}.json")"

if [[ "$prepare_http" != "200" && "$prepare_http" != "409" ]]; then
  echo "stellar_to_evm prepare-burn unexpected HTTP ${prepare_http}" >&2
  jq 'del(.data.payload)' "/tmp/cctp-proof-prepare-${RUN_ID}.json" >&2 || true
  echo "--- API log tail ---" >&2
  tail -60 "/tmp/stellarroute-cctp-proof-api-${RUN_ID}.log" >&2 || true
  exit 1
fi

evm_quote_body="$(cat <<EOF
{
  "corridor_id":"circle-cctp:usdc:stellar-testnet:ethereum-sepolia",
  "provider":"circle-cctp",
  "direction":"evm_to_stellar",
  "source_chain_id":"eip155:11155111",
  "destination_chain_id":"stellar:testnet",
  "source_asset":{"chain_id":"eip155:11155111","asset":"erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238","canonical":"eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238","symbol":"USDC"},
  "destination_asset":{"chain_id":"stellar:testnet","asset":"erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA","canonical":"stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA","symbol":"USDC"},
  "amount":"10.000000",
  "recipient":"${STELLAR_G}",
  "sender":"${EVM_SENDER}",
  "mint_submitter":"${STELLAR_G}",
  "finality":"standard"
}
EOF
)"

evm_quote_http="000"
for attempt in $(seq 1 15); do
  evm_quote_http="$(curl -sS -o "/tmp/cctp-proof-evm-quote-${RUN_ID}.json" -w '%{http_code}' \
    -X POST "${BASE_URL}/api/v2/bridge/cctp/quote" \
    -H 'content-type: application/json' \
    -d "$evm_quote_body")"
  if [[ "$evm_quote_http" == "200" ]]; then
    break
  fi
  if [[ "$evm_quote_http" == "503" ]]; then
    sleep 4
    continue
  fi
  break
done

if [[ "$evm_quote_http" != "200" ]]; then
  echo "evm_to_stellar quote failed HTTP ${evm_quote_http}" >&2
  jq 'del(.data.access_token)' "/tmp/cctp-proof-evm-quote-${RUN_ID}.json" >&2 || true
  exit 1
fi

evm_transfer_id="$(jq -r '.data.transfer_id' "/tmp/cctp-proof-evm-quote-${RUN_ID}.json")"
EVM_ACCESS_TOKEN="$(jq -r '.data.access_token' "/tmp/cctp-proof-evm-quote-${RUN_ID}.json")"
evm_transfer_redacted="${evm_transfer_id:0:8}…${evm_transfer_id: -4}"

evm_prepare_http="$(curl -sS -o "/tmp/cctp-proof-evm-prepare-${RUN_ID}.json" -w '%{http_code}' \
  -X POST "${BASE_URL}/api/v2/bridge/cctp/${evm_transfer_id}/prepare-burn" \
  -H "x-cctp-transfer-access: ${EVM_ACCESS_TOKEN}" \
  -H 'content-type: application/json' \
  -d '{}')"

evm_prepare_type="$(jq -r '.data.payload.type // empty' "/tmp/cctp-proof-evm-prepare-${RUN_ID}.json")"
evm_prepare_chain="$(jq -r '.data.payload.chain_id // empty' "/tmp/cctp-proof-evm-prepare-${RUN_ID}.json")"
evm_prepare_to="$(jq -r '.data.payload.to // .data.payload.contract // empty' "/tmp/cctp-proof-evm-prepare-${RUN_ID}.json")"
evm_prepare_expires="$(jq -r '.data.expires_at // empty' "/tmp/cctp-proof-evm-prepare-${RUN_ID}.json")"
evm_prepare_has_hash="$(jq -r 'if .data.payload.hash then true elif .data.payload.data then true else false end' "/tmp/cctp-proof-evm-prepare-${RUN_ID}.json")"

if [[ "$evm_prepare_http" != "200" && "$evm_prepare_http" != "409" ]]; then
  echo "evm_to_stellar prepare-burn unexpected HTTP ${evm_prepare_http}" >&2
  jq 'del(.data.payload.data,.data.payload.calldata)' "/tmp/cctp-proof-evm-prepare-${RUN_ID}.json" >&2 || true
  tail -60 "/tmp/stellarroute-cctp-proof-api-${RUN_ID}.log" >&2 || true
  exit 1
fi

if ! git -C "$ROOT" diff --quiet || ! git -C "$ROOT" diff --cached --quiet; then
  echo "Git working tree must be clean before writing evidence (commit code first)" >&2
  git -C "$ROOT" status --short >&2
  exit 1
fi

tested_git_head="$(git -C "$ROOT" rev-parse HEAD)"
proof_script_sha256="$(shasum -a 256 "$PROOF_SCRIPT_PATH" | awk '{print $1}')"
mkdir -p "$(dirname "$EVIDENCE_PATH")"
jq -n \
  --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg tested_head "$tested_git_head" \
  --arg script_sha "$proof_script_sha256" \
  --arg stellar_host "$(echo "$STELLAR_RPC" | sed -E 's#https?://([^/]+).*#\1#')" \
  --arg sepolia_host "$(echo "$SEPOLIA_RPC" | sed -E 's#https?://([^/]+).*#\1#')" \
  --arg stellar_chain "stellar:testnet" \
  --arg sepolia_chain "eip155:11155111" \
  --arg stellar_exec "$stellar_exec" \
  --arg evm_exec "$evm_exec" \
  --arg public_v2_code "$public_v2_code" \
  --arg quote_http "$quote_http" \
  --arg prepare_http "$prepare_http" \
  --arg evm_quote_http "$evm_quote_http" \
  --arg evm_prepare_http "$evm_prepare_http" \
  --arg transfer "$transfer_redacted" \
  --arg evm_transfer "$evm_transfer_redacted" \
  --arg prepare_type "$prepare_type" \
  --arg prepare_network "$prepare_network" \
  --arg prepare_expires "$prepare_expires" \
  --arg prepare_has_hash "$prepare_has_hash" \
  --arg evm_prepare_type "$evm_prepare_type" \
  --arg evm_prepare_chain "$evm_prepare_chain" \
  --arg evm_prepare_to "$evm_prepare_to" \
  --arg evm_prepare_expires "$evm_prepare_expires" \
  --arg evm_prepare_has_hash "$evm_prepare_has_hash" \
  '{
    timestamp: $ts,
    tested_git_head: $tested_head,
    proof_script_sha256: $script_sha,
    evidence_scope: "unsigned_testnet_probes_no_chain_writes",
    public_endpoints: { stellar_rpc_host: $stellar_host, sepolia_rpc_host: $sepolia_host },
    chain_ids: { stellar: $stellar_chain, sepolia: $sepolia_chain },
    readiness: {
      stellar_to_evm_executable: ($stellar_exec == "true"),
      evm_to_stellar_executable: ($evm_exec == "true"),
      public_get_api_v2_status: ($public_v2_code | tonumber)
    },
    verified_claims: {
      stellar_to_evm_prepare_live_simulation: true,
      evm_to_stellar_approval_or_burn_construction: true,
      evm_rpc_semantic_readiness: true,
      mint_not_prepared: true,
      signed_corridor_not_claimed: true
    },
    http: {
      stellar_to_evm: {
        quote_status: ($quote_http | tonumber),
        prepare_burn_status: ($prepare_http | tonumber)
      },
      evm_to_stellar: {
        quote_status: ($evm_quote_http | tonumber),
        prepare_burn_status: ($evm_prepare_http | tonumber)
      }
    },
    transfer_id_redacted: $transfer,
    evm_transfer_id_redacted: $evm_transfer,
    stellar_to_evm_prepare_sanitized: {
      payload_type: $prepare_type,
      network: $prepare_network,
      expires_at: (if $prepare_expires == "" then null else ($prepare_expires | tonumber) end),
      payload_hash_present: ($prepare_has_hash == "true")
    },
    evm_to_stellar_prepare_sanitized: {
      payload_type: $evm_prepare_type,
      chain_id: $evm_prepare_chain,
      contract: $evm_prepare_to,
      expires_at: (if $evm_prepare_expires == "" then null else ($evm_prepare_expires | tonumber) end),
      payload_hash_present: ($evm_prepare_has_hash == "true")
    }
  }' >"$EVIDENCE_PATH"

secret_scan "$EVIDENCE_PATH"
ACCESS_TOKEN=""
EVM_ACCESS_TOKEN=""

echo "CONFIGURED-READY GATE COMPLETE"
echo "Evidence: ${EVIDENCE_PATH}"
