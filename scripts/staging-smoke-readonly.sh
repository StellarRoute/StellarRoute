#!/usr/bin/env bash
# scripts/staging-smoke-readonly.sh — Wave 0 / issue #1275
# Read-only staging smoke: hits /health, /health/deps, /api/v1/pairs, and one /api/v1/quote pair.
# Never POSTs /swap/prepare or mutates state. Purely additive — no behavior changes.
#
# Usage:
#   STAGING_API_BASE_URL=https://api.example.com ./scripts/staging-smoke-readonly.sh
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
PAIRS_BUDGET_MS="${PAIRS_BUDGET_MS:-3000}"
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

echo "Staging smoke (read-only) against ${ORIGIN}"

# Health endpoints
check_endpoint "health" "${ORIGIN}/health" "${HEALTH_BUDGET_MS}"
check_endpoint "health_deps" "${ORIGIN}/health/deps" "${DEPS_BUDGET_MS}"

# Pairs endpoint (read-only, no prepare/submit)
PAIRS_URL="${ORIGIN}/api/v1/pairs"
check_endpoint "pairs" "${PAIRS_URL}" "${PAIRS_BUDGET_MS}"

# One quote pair (read-only, no prepare/submit)
QUOTE_URL="${ORIGIN}/api/v1/quote/${QUOTE_BASE}/${QUOTE_ASSET}?amount=${QUOTE_AMOUNT}"
check_endpoint "quote" "${QUOTE_URL}" "${QUOTE_BUDGET_MS}"

echo "Staging smoke (read-only) passed."