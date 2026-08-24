#!/usr/bin/env bash
# Opt-in read-only CCTP HTTP smoke: quote gate + unsigned prepare path probe.
# Does not sign or submit chain transactions. Sanitizes output (no access tokens).
set -euo pipefail

BASE_URL="${CCTP_SMOKE_BASE_URL:-http://127.0.0.1:3000}"
ENABLED="${CCTP_ENABLED:-false}"

echo "CCTP HTTP smoke → ${BASE_URL} (CCTP_ENABLED=${ENABLED})"

info="$(curl -fsS "${BASE_URL}/api/v2")"
echo "${info}" | jq 'del(.data.supported_corridors[].source_asset, .data.supported_corridors[].destination_asset) | .data | {bridge_settlement_executable, corridor_count:(.supported_corridors|length)}'

quote_body='{
  "corridor_id":"circle-cctp:usdc:stellar-testnet:ethereum-sepolia",
  "provider":"circle-cctp",
  "direction":"evm_to_stellar",
  "source_chain_id":"eip155:11155111",
  "destination_chain_id":"stellar:testnet",
  "source_asset":{"chain_id":"eip155:11155111","asset":"erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238","canonical":"eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238","symbol":"USDC"},
  "destination_asset":{"chain_id":"stellar:testnet","asset":"erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA","canonical":"stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA","symbol":"USDC"},
  "amount":"10.000000",
  "recipient":"GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
  "mint_submitter":"GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
  "finality":"standard"
}'

set +e
quote_resp="$(curl -sS -w '\n%{http_code}' -X POST "${BASE_URL}/api/v2/bridge/cctp/quote" \
  -H 'content-type: application/json' \
  -H 'idempotency-key: smoke-readonly-1' \
  -d "${quote_body}")"
set -e
quote_http="$(echo "${quote_resp}" | tail -n1)"
quote_json="$(echo "${quote_resp}" | sed '$d')"
echo "quote HTTP ${quote_http}"
echo "${quote_json}" | jq 'if .data.access_token then .data |= del(.access_token) else . end'

if [[ "${quote_http}" == "200" ]]; then
  transfer_id="$(echo "${quote_json}" | jq -r '.data.transfer_id')"
  echo "prepare-burn probe for transfer (redacted id suffix)"
  curl -sS -o /dev/null -w "prepare-burn HTTP %{http_code}\n" \
    -X POST "${BASE_URL}/api/v2/bridge/cctp/${transfer_id}/prepare-burn" \
    -H "x-cctp-transfer-access: REDACTED"
fi

echo "smoke complete (no writes/submits performed)"
