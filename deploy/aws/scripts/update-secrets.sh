#!/usr/bin/env bash
# Merge operator secrets into the Terraform-managed Secrets Manager secret.
# Run from anywhere; requires jq, aws, openssl, and a prior terraform apply.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
TF_DIR="${ROOT}/deploy/aws/terraform"
DEPLOYMENTS="${ROOT}/config/deployments/testnet.json"

cd "${TF_DIR}"
SECRET_ID="$(terraform output -raw secrets_name)"
DB_URL="$(terraform output -raw database_url)"
REDIS_URL="$(terraform output -raw redis_url)"
ROUTER="$(jq -r .router_contract_id "${DEPLOYMENTS}")"
ADMIN="${ADMIN_AUTH_TOKEN:-$(openssl rand -hex 32)}"
CORS="${CORS_ALLOWED_ORIGINS:-https://www.stellarroute.app,https://stellarroute.app,https://stellarroute-frontend.vercel.app}"

tmp="$(mktemp)"
trap 'rm -f "${tmp}"' EXIT

jq -n \
  --arg db "${DB_URL}" \
  --arg redis "${REDIS_URL}" \
  --arg admin "${ADMIN}" \
  --arg router "${ROUTER}" \
  --arg cors "${CORS}" \
  --arg amm "${AMM_POOLS:-}" \
  '{
    DATABASE_URL: $db,
    REDIS_URL: $redis,
    ADMIN_AUTH_TOKEN: $admin,
    ROUTER_CONTRACT_ADDRESS: $router,
    SOROBAN_RPC_URL: "https://soroban-testnet.stellar.org",
    STELLAR_HORIZON_URL: "https://horizon-testnet.stellar.org",
    CORS_ALLOWED_ORIGINS: $cors,
    PUBLIC_GET_ROUTES: "/api/v1/quote,/api/v1/pairs,/api/v1/markets,/api/v1/orderbook,/api/v1/routes,/api/v1/price-history,/health",
    AMM_POOLS: $amm,
    RUST_LOG: "info,stellarroute_api=info,stellarroute_indexer=info"
  }' > "${tmp}"

aws secretsmanager put-secret-value \
  --secret-id "${SECRET_ID}" \
  --secret-string "file://${tmp}"

echo "Updated secret ${SECRET_ID}"
echo "Force a new ECS deployment so tasks pick up rotated values."
