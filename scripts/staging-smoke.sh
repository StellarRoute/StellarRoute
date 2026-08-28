#!/usr/bin/env bash
# scripts/staging-smoke.sh — Wave 0 / issue #1037
# Hits staging /health, /health/deps, and one /api/v1/quote pair.
# Usage:
#   STAGING_API_BASE_URL=https://api.example.com ./scripts/staging-smoke.sh
# Base URL may include or omit /api/v1.
set -euo pipefail

BASE_URL="${STAGING_API_BASE_URL:-${TARGET_URL:-}}"
if [[ -z "${BASE_URL}" ]]; then
  echo "Set STAGING_API_BASE_URL to the public API origin (e.g. https://api.example.com)" >&2
  exit 1
fi

# Trim trailing slash and optional /api/v1 suffix for root probes.
ORIGIN="${BASE_URL%/}"
ORIGIN="${ORIGIN%/api/v1}"

QUOTE_BASE="${PROBE_BASE_ASSET:-BTC:GDMVY5CPSEY6IDQBEX7KMJSOVFNHMOMT5QY4MTOCSDFORV24AOFYDDGS}"
QUOTE_ASSET="${PROBE_QUOTE_ASSET:-EXT:GDMVY5CPSEY6IDQBEX7KMJSOVFNHMOMT5QY4MTOCSDFORV24AOFYDDGS}"
QUOTE_AMOUNT="${PROBE_AMOUNT:-0.01}"
HEALTH_BUDGET_MS="${HEALTH_BUDGET_MS:-2000}"
DEPS_BUDGET_MS="${DEPS_BUDGET_MS:-3000}"
QUOTE_BUDGET_MS="${QUOTE_BUDGET_MS:-5000}"

tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

check_endpoint() {
  local name="$1"
  local url="$2"
  local budget_ms="$3"
  local out="${tmpdir}/${name}.body"
  local metrics="${tmpdir}/${name}.metrics"

  curl -sS -o "${out}" -w '%{http_code} %{time_total}' \
    --connect-timeout 10 --max-time 30 \
    "${url}" >"${metrics}"

  local code time_s time_ms
  code="$(awk '{print $1}' "${metrics}")"
  time_s="$(awk '{print $2}' "${metrics}")"
  time_ms="$(python3 -c "print(int(float('${time_s}') * 1000))")"

  if [[ "${code}" != 2* ]]; then
    echo "FAIL ${name}: HTTP ${code} from ${url}" >&2
    head -c 500 "${out}" >&2 || true
    echo >&2
    return 1
  fi
  if (( time_ms > budget_ms )); then
    echo "FAIL ${name}: latency ${time_ms}ms exceeds budget ${budget_ms}ms" >&2
    return 1
  fi
  echo "OK   ${name}: HTTP ${code} in ${time_ms}ms (budget ${budget_ms}ms)"
}

echo "Staging smoke against ${ORIGIN}"

check_endpoint "health" "${ORIGIN}/health" "${HEALTH_BUDGET_MS}"
check_endpoint "health_deps" "${ORIGIN}/health/deps" "${DEPS_BUDGET_MS}"

QUOTE_URL="${ORIGIN}/api/v1/quote/${QUOTE_BASE}/${QUOTE_ASSET}?amount=${QUOTE_AMOUNT}"
check_endpoint "quote" "${QUOTE_URL}" "${QUOTE_BUDGET_MS}"

API_V2_BUDGET_MS="${API_V2_BUDGET_MS:-3000}"
check_endpoint "api_v2" "${ORIGIN}/api/v2" "${API_V2_BUDGET_MS}"

python3 - "${tmpdir}/api_v2.body" <<'PY'
import json, sys
path = sys.argv[1]
with open(path) as f:
    data = json.load(f)
payload = data.get("data", data)
if not isinstance(payload, dict):
    print("FAIL api_v2: missing data object", file=sys.stderr)
    sys.exit(1)
if "bridge_settlement_executable" not in payload:
    print("FAIL api_v2: missing bridge_settlement_executable", file=sys.stderr)
    sys.exit(1)
corridors = payload.get("supported_corridors")
if not isinstance(corridors, list):
    print("FAIL api_v2: supported_corridors must be a list", file=sys.stderr)
    sys.exit(1)
print(
    "OK   api_v2: shape valid "
    f"(bridge_settlement_executable={payload['bridge_settlement_executable']}, "
    f"corridors={len(corridors)})"
)
PY

python3 - "${tmpdir}/quote.body" <<'PY'
import json, sys
path = sys.argv[1]
with open(path) as f:
    raw = f.read()
try:
    data = json.loads(raw)
except json.JSONDecodeError as e:
    print(f"FAIL quote: invalid JSON ({e})", file=sys.stderr)
    sys.exit(1)

payload = data.get("data", data)
if isinstance(payload, dict) and "quote" in payload and isinstance(payload["quote"], dict):
    quote = payload["quote"]
else:
    quote = payload if isinstance(payload, dict) else {}

has_amount = any(
    k in quote and quote[k] is not None
    for k in (
        "amount_in",
        "amount_out",
        "amount",
        "input_amount",
        "output_amount",
        "sell_amount",
        "buy_amount",
        "expected_output",
        "price",
    )
)
has_route = any(k in quote for k in ("route", "path", "routes", "legs", "hops"))
if not has_amount and not has_route:
    err = data.get("error") or (payload.get("error") if isinstance(payload, dict) else None)
    print(
        "FAIL quote: payload missing amount/route fields; "
        f"keys={list(quote)[:20]} error={err!r}",
        file=sys.stderr,
    )
    sys.exit(1)

print("OK   quote: payload contains amount/route fields")
PY

echo "Staging smoke passed."
