#!/usr/bin/env bash
#
# Export Soroban gas benchmark results to CSV.
#
# Usage:
#   ./scripts/export-gas-benchmarks.sh [output.csv]
#
# Runs every bench_* and stress_test_* test in the contracts crate with
# --nocapture, collects the JSON lines emitted to stderr, and writes a CSV
# file with columns: name, cpu_cost, threshold, passed.
#
# The script exits non-zero if any benchmark assertion fails.
set -euo pipefail

OUTPUT="${1:-gas_benchmarks.csv}"
TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

echo "Running gas benchmarks (release mode) …"

# Each benchmark test emits a JSON line to stderr via emit_benchmark().
# Capture stderr (JSON lines) while letting stdout (test harness) go to the
# terminal.  --test-threads=1 ensures deterministic, sequential execution.
cargo test \
  -p stellarroute-contracts \
  --release \
  --bench_ \
  --stress_test_ \
  -- --nocapture --test-threads=1 2>"$TMPFILE" || {
    echo "ERROR: one or more benchmark assertions failed." >&2
    cat "$TMPFILE" >&2
    exit 1
  }

# Filter JSON lines emitted by emit_benchmark and write CSV.
echo "name,cpu_cost,threshold,passed" > "$OUTPUT"
grep '^{\"name\"' "$TMPFILE" | while IFS= read -r line; do
  name=$(echo "$line"   | sed 's/.*"name":"\([^"]*\)".*/\1/')
  cost=$(echo "$line"   | sed 's/.*"cpu_cost":\([0-9]*\).*/\1/')
  thr=$(echo "$line"    | sed 's/.*"threshold":\([0-9]*\).*/\1/')
  passed=$(echo "$line" | sed 's/.*"passed":\([^,}]*\).*/\1/')
  echo "${name},${cost},${thr},${passed}"
done >> "$OUTPUT"

echo ""
echo "✅ Benchmark results written to $OUTPUT"
echo ""
column -t -s',' "$OUTPUT"
