#!/usr/bin/env bash
#
# Export redacted StellarRoute audit logs to stdout (dry-run), file, or object storage.
#
# Usage:
#   ./scripts/export-audit-logs.sh --dry-run
#   ./scripts/export-audit-logs.sh --table route --output-file audit_export.jsonl
#   ./scripts/export-audit-logs.sh --s3-bucket my-audit-bucket --s3-prefix audit/2026/
#
# Options:
#   --dry-run, -d       Output redacted JSON lines to stdout (dry-run mode)
#   --table <TARGET>    Target table: route | swap | all (default: all)
#   --from <TIMESTAMP>  ISO8601 start timestamp (e.g. 2026-08-01T00:00:00Z)
#   --to <TIMESTAMP>    ISO8601 end timestamp (e.g. 2026-08-30T23:59:59Z)
#   --limit <N>         Maximum records per table (default: 10000)
#   --output-file <PATH> Local JSON lines output file path
#   --s3-bucket <NAME>  Target S3 bucket for object storage export
#   --s3-prefix <PATH>  S3 folder key prefix (default: "audit/")
#   --s3-endpoint <URL> Custom S3 endpoint URL (MinIO, R2, etc.)
#   --db-url <URL>      PostgreSQL database URL (defaults to DATABASE_URL)
#   --help, -h          Show this help message
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

show_help() {
  cat <<'EOF'
export-audit-logs.sh — Export redacted StellarRoute audit logs

USAGE:
  ./scripts/export-audit-logs.sh [FLAGS] [OPTIONS]

FLAGS:
  -d, --dry-run       Print redacted NDJSON logs directly to stdout
  -h, --help          Show this help text

OPTIONS:
  --table <TARGET>    Target table: route | swap | all (default: all)
  --from <ISO8601>    Start timestamp filter
  --to <ISO8601>      End timestamp filter
  --limit <N>         Maximum records limit (default: 10000)
  --output-file <FILE> Local destination file path (.jsonl)
  --s3-bucket <BUCKET> Object storage bucket name
  --s3-prefix <PREFIX> S3 key prefix folder (default: "audit/")
  --s3-endpoint <URL>  Custom S3 endpoint URL
  --db-url <URL>      Database URL (defaults to $DATABASE_URL)
EOF
}

# Parse basic CLI flag checks for --help
for arg in "$@"; do
  if [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
    show_help
    exit 0
  fi
done

echo "Running audit log export tool..." >&2

cargo run --manifest-path "${REPO_ROOT}/Cargo.toml" -p stellarroute-api --bin audit-export -- "$@"
