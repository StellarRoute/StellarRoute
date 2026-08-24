#!/usr/bin/env bash
# ===========================================================================
# StellarRoute SLO Probe Runner
# ===========================================================================
# Runs synthetic probes against the StellarRoute API to validate SLO
# compliance for quote latency and error rate targets.
#
# Probe definitions are maintained in config/slo.yaml. This script
# implements the executable probes referenced in that configuration.
#
# Usage:
#   ./scripts/slo-probe.sh --base-url https://api.stellarroute.io
#   ./scripts/slo-probe.sh --base-url http://localhost:3000 --verbose
#
# Exit codes:
#   0 - All probes passed
#   1 - One or more probes failed
# ===========================================================================
set -euo pipefail

# ── Defaults ──────────────────────────────────────────────────────────────────
BASE_URL=""
VERBOSE=false
QUIET=false
TIMEOUT_SECS=10

# ── Color helpers ──────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() { echo -e "  ${GREEN}✓ PASS${NC} $1"; }
fail() { echo -e "  ${RED}✗ FAIL${NC} $1"; }
warn() { echo -e "  ${YELLOW}⚠ WARN${NC} $1"; }
info() { echo -e "  ${NC}• $1${NC}"; }

# ── Argument parsing ──────────────────────────────────────────────────────────
usage() {
    cat <<EOF
Usage: $(basename "$0") --base-url URL [options]

Options:
  --base-url URL   Target API base URL (required)
  --verbose        Print response bodies and detailed output
  --quiet          Suppress all output except pass/fail summary
  --timeout SECS   HTTP request timeout in seconds (default: $TIMEOUT_SECS)
  --help           Show this help message
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --base-url)    BASE_URL="$2";  shift 2 ;;
        --verbose)     VERBOSE=true;   shift   ;;
        --quiet)       QUIET=true;     shift   ;;
        --timeout)     TIMEOUT_SECS="$2"; shift 2 ;;
        --help)        usage                   ;;
        *) echo "Unknown option: $1"; usage    ;;
    esac
done

if [[ -z "$BASE_URL" ]]; then
    echo "ERROR: --base-url is required"
    usage
fi

# Strip trailing slash
BASE_URL="${BASE_URL%/}"

# ── Probe definitions ─────────────────────────────────────────────────────────
# These correspond to the probes defined in config/slo.yaml.
#
# Smoke test values are for CI / synthetic monitoring. Load probe values
# target the SLO thresholds from config/slo.yaml:
#   - P95 latency < 500ms
#   - P99 latency < 2s
#   - Error rate  < 1%

PROBES=(
    # name|method|path|expected_status|max_latency_ms|query_params
    "quote_smoke_xlm_usdc|GET|/api/v1/quote/XLM/USDC|200|2000|amount=100.0"
    "quote_smoke_usdc_xlm|GET|/api/v1/quote/USDC/XLM|200|2000|amount=100.0"
    "quote_smoke_xlm_eurc|GET|/api/v1/quote/XLM/EURC|200|2000|amount=100.0"
    "route_smoke_xlm_usdc|GET|/api/v1/route/XLM/USDC|200|5000|amount=100.0"
)

# Load probe thresholds (P50/P95/P99) — used for summary only; actual
# latency checks use individual max_latency_ms above.

# ── Results ───────────────────────────────────────────────────────────────────
TOTAL=0
PASSED=0
FAILED=0
FAILURES=""

# ── Helpers ────────────────────────────────────────────────────────────────────

# Run a single HTTP probe and validate status + latency.
run_probe() {
    local name="$1"
    local method="$2"
    local path="$3"
    local expected_status="$4"
    local max_latency_ms="$5"
    local query_params="${6:-}"

    TOTAL=$((TOTAL + 1))

    [[ "$QUIET" == false ]] && info "Probe: $name"
    [[ "$VERBOSE" == true ]] && info "  $method $BASE_URL$path?$query_params"

    local url="$BASE_URL$path"
    if [[ -n "$query_params" ]]; then
        url="$url?$query_params"
    fi

    local start_ms end_ms elapsed_ms http_code response_body

    start_ms=$(date +%s%3N)

    # Execute the HTTP request
    response_body=$(curl -sS -w "%{http_code}" \
        --max-time "$TIMEOUT_SECS" \
        -X "$method" \
        "$url" 2>&1) || {
        local exit_code=$?
        [[ "$QUIET" == false ]] && fail "$name — curl failed with exit code $exit_code"
        FAILED=$((FAILED + 1))
        FAILURES="$FAILURES  - $name (curl error $exit_code)\n"
        return
    }

    end_ms=$(date +%s%3N)
    elapsed_ms=$(( end_ms - start_ms ))

    # Extract HTTP status code (last 3 chars of response)
    http_code="${response_body: -3}"
    # Extract response body (everything except last 3 chars)
    local body_len=${#response_body}
    local resp_body="${response_body:0:body_len-3}"

    [[ "$VERBOSE" == true ]] && info "  HTTP $http_code, ${elapsed_ms}ms"

    # Validate HTTP status
    if [[ "$http_code" != "$expected_status" ]]; then
        [[ "$QUIET" == false ]] && fail "$name — expected status $expected_status, got $http_code"
        [[ "$VERBOSE" == true ]] && info "  Response: $resp_body"
        FAILED=$((FAILED + 1))
        FAILURES="$FAILURES  - $name (HTTP $http_code, expected $expected_status)\n"
        return
    fi

    # Validate latency
    if [[ "$elapsed_ms" -gt "$max_latency_ms" ]]; then
        [[ "$QUIET" == false ]] && fail "$name — ${elapsed_ms}ms exceeds max ${max_latency_ms}ms"
        FAILED=$((FAILED + 1))
        FAILURES="$FAILURES  - $name (${elapsed_ms}ms > ${max_latency_ms}ms)\n"
        return
    fi

    [[ "$QUIET" == false ]] && pass "$name — HTTP $http_code, ${elapsed_ms}ms"
    PASSED=$((PASSED + 1))
}

# ── Execution ──────────────────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  StellarRoute SLO Probe Runner"
echo "  Target: $BASE_URL"
echo "  Timeout: ${TIMEOUT_SECS}s"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Check that curl is available
if ! command -v curl &>/dev/null; then
    echo "ERROR: curl is required but not installed."
    exit 1
fi

# Run each probe
for probe_def in "${PROBES[@]}"; do
    IFS='|' read -r name method path expected_status max_latency_ms query_params <<< "$probe_def"
    run_probe "$name" "$method" "$path" "$expected_status" "$max_latency_ms" "$query_params"
done

# ── Summary ────────────────────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Results: $PASSED/$TOTAL passed, $FAILED failed"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if [[ -n "$FAILURES" ]]; then
    echo -e "Failed probes:\n$FAILURES"
fi

if [[ "$FAILED" -gt 0 ]]; then
    exit 1
fi

exit 0
