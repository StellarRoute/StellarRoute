#!/usr/bin/env bash
# Overnight / long-running fuzz for stellarroute-contracts router entrypoints.
# See audit/fuzzing.md for details.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CASES="${PROPTEST_CASES:-500000}"
echo "==> Fuzzing stellarroute-contracts with PROPTEST_CASES=${CASES}"
echo "==> Commit: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "==> Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

PROPTEST_CASES="${CASES}" cargo test -p stellarroute-contracts fuzz_ -- --nocapture

echo "==> Finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "==> Record results in audit/fuzz-runs/YYYY-MM-DD.md"
