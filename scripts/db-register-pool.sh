#!/usr/bin/env bash
# Simple admin script to register/unregister pools in Postgres `amm_pools` table.
# Usage:
#   ./scripts/db-register-pool.sh add <POOL_ADDRESS> [network]
#   ./scripts/db-register-pool.sh remove <POOL_ADDRESS>

set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 add|remove POOL_ADDRESS [network]"
  exit 2
fi

OP=$1
POOL=$2
NETWORK=${3:-mainnet}

DB_URL="${DATABASE_URL:-postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute}"

psql "$DB_URL" -v ON_ERROR_STOP=1 -q -c "\
BEGIN;
\
-- Add
\"" >/dev/null 2>&1 || true

if [[ "$OP" == "add" ]]; then
  psql "$DB_URL" -v ON_ERROR_STOP=1 -c "INSERT INTO amm_pools (pool_address, network, active, metadata) VALUES ('${POOL}', '${NETWORK}', true, '{}'::jsonb) ON CONFLICT (pool_address) DO UPDATE SET network=EXCLUDED.network, active=true, updated_at=now();"
  echo "Registered ${POOL} (network=${NETWORK})"
elif [[ "$OP" == "remove" ]]; then
  psql "$DB_URL" -v ON_ERROR_STOP=1 -c "DELETE FROM amm_pools WHERE pool_address = '${POOL}';"
  echo "Unregistered ${POOL}"
else
  echo "Unknown op: ${OP}"; exit 2
fi
