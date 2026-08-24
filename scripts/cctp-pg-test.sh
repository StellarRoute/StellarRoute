#!/usr/bin/env bash
# Disposable Postgres for CCTP store + prepare-lock integration tests.
# Prefers Docker when healthy; falls back to local initdb or existing local server.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUN_ID="${CCTP_PG_RUN_ID:-$$}"
CONTAINER="stellarroute-cctp-pg-test-${RUN_ID}"
PORT="${CCTP_PG_PORT:-55432}"
DB_NAME="stellarroute_cctp_test_${RUN_ID}"
PGDATA="${CCTP_PGDATA:-/tmp/stellarroute-cctp-pg-${RUN_ID}}"
LOCAL_PG_PID=""
USE_DOCKER=0
USE_EXISTING_LOCAL=0
LOCAL_PG_SUPERUSER="${LOCAL_PG_SUPERUSER:-$USER}"
LOCAL_PG_HOST="${LOCAL_PG_HOST:-127.0.0.1}"
LOCAL_PG_PORT="${LOCAL_PG_PORT:-5432}"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

# Sanitized connection metadata (never log database URLs or credentials).
PG_BACKEND=""
PG_HOST=""
PG_PORT_META=""
PG_USER=""

assert_script_output_redacted() {
  local script="$ROOT/scripts/cctp-pg-test.sh"
  if grep -v '^[[:space:]]*#' "$script" \
    | grep -E '(^|[[:space:]])(echo|printf).+(\$TEST_DATABASE_URL|postgres://)' \
    >/dev/null 2>&1; then
    echo "Script redaction assertion failed: URL or credential echo detected in $script" >&2
    exit 1
  fi
}

print_pg_meta() {
  echo "Postgres backend=${PG_BACKEND} host=${PG_HOST} port=${PG_PORT_META} database=${DB_NAME}"
}

on_err() {
  echo "CCTP PG harness failed (backend=${PG_BACKEND:-unknown} host=${PG_HOST:-?} port=${PG_PORT_META:-?} database=${DB_NAME})" >&2
}
trap on_err ERR

psql_admin() {
  psql -h "$LOCAL_PG_HOST" -p "$LOCAL_PG_PORT" -U "$LOCAL_PG_SUPERUSER" -d postgres "$@"
}

psql_test_db() {
  psql -h "$PG_HOST" -p "$PG_PORT_META" -U "$PG_USER" -d "$DB_NAME" "$@"
}

cleanup() {
  if [[ "$USE_DOCKER" -eq 1 ]]; then
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  elif [[ "$USE_EXISTING_LOCAL" -eq 1 ]]; then
    psql_admin -c "DROP DATABASE IF EXISTS ${DB_NAME};" >/dev/null 2>&1 || true
  elif [[ -n "$LOCAL_PG_PID" ]] && [[ -d "$PGDATA" ]]; then
    LC_ALL=en_US.UTF-8 LANG=en_US.UTF-8 pg_ctl -D "$PGDATA" -m fast stop >/dev/null 2>&1 || true
    rm -rf "$PGDATA"
  fi
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
    echo "Applying $(basename "$f")"
    if ! psql_test_db -v ON_ERROR_STOP=1 -f "$f" >/dev/null 2>&1; then
      echo "Migration failed: $(basename "$f") (backend=${PG_BACKEND} host=${PG_HOST} port=${PG_PORT_META} database=${DB_NAME})" >&2
      return 1
    fi
  done
}

wait_for_pg() {
  local tries=0
  until pg_isready -h "$PG_HOST" -p "$PG_PORT_META" >/dev/null 2>&1; do
    tries=$((tries + 1))
    if [[ "$tries" -ge 60 ]]; then
      echo "Postgres did not become ready (backend=${PG_BACKEND} host=${PG_HOST} port=${PG_PORT_META})" >&2
      return 1
    fi
    sleep 1
  done
}

start_docker_pg() {
  if ! command -v docker >/dev/null 2>&1; then
    return 1
  fi
  if ! docker info >/dev/null 2>&1; then
    return 1
  fi
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  if ! docker run -d --name "$CONTAINER" -e POSTGRES_PASSWORD=postgres -p "${PORT}:5432" postgres:16-alpine >/dev/null 2>&1; then
    return 1
  fi
  for _ in $(seq 1 60); do
    if docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done
  docker exec "$CONTAINER" psql -U postgres -c "CREATE DATABASE ${DB_NAME};" >/dev/null
  PG_BACKEND="docker"
  PG_HOST="127.0.0.1"
  PG_PORT_META="$PORT"
  PG_USER="postgres"
  export PGPASSWORD="postgres"
  export TEST_DATABASE_URL="postgres://${PG_USER}:${PGPASSWORD}@${PG_HOST}:${PG_PORT_META}/${DB_NAME}"
  USE_DOCKER=1
  print_pg_meta
}

start_local_pg() {
  if ! command -v initdb >/dev/null 2>&1 || ! command -v pg_ctl >/dev/null 2>&1; then
    return 1
  fi
  rm -rf "$PGDATA"
  if ! LC_ALL=en_US.UTF-8 LANG=en_US.UTF-8 initdb -D "$PGDATA" -U postgres -A trust --encoding=UTF8 >/dev/null 2>&1; then
    return 1
  fi
  if ! LC_ALL=en_US.UTF-8 LANG=en_US.UTF-8 pg_ctl -D "$PGDATA" -o "-p ${PORT}" -w start >/dev/null 2>&1; then
    return 1
  fi
  LOCAL_PG_PID=1
  PG_BACKEND="initdb"
  PG_HOST="127.0.0.1"
  PG_PORT_META="$PORT"
  PG_USER="postgres"
  unset PGPASSWORD
  wait_for_pg
  psql -h "$PG_HOST" -p "$PG_PORT_META" -U "$PG_USER" -d postgres -c "CREATE DATABASE ${DB_NAME};" >/dev/null
  export TEST_DATABASE_URL="postgres://${PG_USER}@${PG_HOST}:${PG_PORT_META}/${DB_NAME}"
  print_pg_meta
}

start_existing_local_pg() {
  if ! command -v psql >/dev/null 2>&1; then
    return 1
  fi
  if ! psql_admin -c "SELECT 1" >/dev/null 2>&1; then
    return 1
  fi
  psql_admin -c "DROP DATABASE IF EXISTS ${DB_NAME};" >/dev/null 2>&1 || true
  psql_admin -c "CREATE DATABASE ${DB_NAME};" >/dev/null
  PG_BACKEND="existing-local"
  PG_HOST="$LOCAL_PG_HOST"
  PG_PORT_META="$LOCAL_PG_PORT"
  PG_USER="$LOCAL_PG_SUPERUSER"
  unset PGPASSWORD
  export TEST_DATABASE_URL="postgres://${PG_USER}@${PG_HOST}:${PG_PORT_META}/${DB_NAME}"
  USE_EXISTING_LOCAL=1
  print_pg_meta
}

assert_script_output_redacted

if start_docker_pg; then
  :
elif start_local_pg; then
  :
elif start_existing_local_pg; then
  :
else
  echo "Failed to start disposable Postgres (Docker unhealthy, initdb failed, no local server)" >&2
  exit 1
fi

apply_migrations

cd "$ROOT"
run_pg_tests() {
  local label="$1"
  echo "=== PG test run: $label ==="
  export CCTP_IDEMPOTENCY_LEASE_SECS="${CCTP_IDEMPOTENCY_LEASE_SECS:-2}"
  env -u CCTP_ENABLED -u CCTP_ACCESS_TOKEN_HMAC_KEY \
    cargo test -p stellarroute-api --test cctp_store_integration -- --ignored --nocapture
  env -u CCTP_ENABLED -u CCTP_ACCESS_TOKEN_HMAC_KEY \
    cargo test -p stellarroute-api --test cctp_prepare_lock_pg -- --ignored --nocapture
  env -u CCTP_ENABLED -u CCTP_ACCESS_TOKEN_HMAC_KEY \
    cargo test -p stellarroute-api --test cctp_prepare_lock_pg memory_pg_parity_core_semantics -- --nocapture
  env -u CCTP_ENABLED -u CCTP_ACCESS_TOKEN_HMAC_KEY \
    cargo test -p stellarroute-api --test api_v2_cctp_http_pg -- --ignored --test-threads=1 --nocapture
}

export CARGO_TARGET_DIR
run_pg_tests "pass-1"
run_pg_tests "pass-2"

print_pg_meta
echo "CCTP PG tests passed twice (store + prepare-lock + memory/PG parity + HTTP PG)"
