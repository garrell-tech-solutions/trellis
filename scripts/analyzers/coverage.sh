#!/usr/bin/env bash
# Analyzer Contract wrapper around cargo-llvm-cov.
# Usage: coverage.sh <path>
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

THRESHOLD=80
TARGET="${1:-.}"

if pkg="$(pkg_for_path "$TARGET" 2>/dev/null)"; then
  SCOPE=(-p "$pkg")
  REPORT_SCOPE=(-p "$pkg")
else
  SCOPE=(--workspace)
  # `cargo llvm-cov report` has no --workspace: with no package filter it
  # renders everything in the profile, which is what --workspace collected.
  REPORT_SCOPE=()
fi

RAW_FILE="$(mktemp)"
trap 'rm -f "$RAW_FILE"' EXIT

# The instrumented run itself, shared with crap.sh -- see lib.sh, which also
# carries the reasoning for --include-ignored and --ignore-run-fail. Taken here
# if nothing has taken it yet, so this analyzer still works standalone.
llvm_cov_profile_ensure "${SCOPE[*]}" "${SCOPE[@]}"

# Rendering only. `report` runs no tests; it exports the profile above.
cargo llvm-cov report "${REPORT_SCOPE[@]}" --json --summary-only \
  >"$RAW_FILE" 2>/dev/null

python3 - "$THRESHOLD" "$RAW_FILE" <<'PYEOF'
import json, sys

threshold, raw_file = float(sys.argv[1]), sys.argv[2]
with open(raw_file) as fh:
    data = json.load(fh)
totals = data["data"][0]["totals"]["lines"]
percent = totals["percent"]

violations = []
for f in data["data"][0]["files"]:
    p = f["summary"]["lines"]["percent"]
    if p < threshold:
        violations.append({
            "file": f["filename"],
            "coverage_percent": round(p, 2),
        })

result = {
    "tool": "cargo-llvm-cov",
    "metric": "coverage",
    "threshold": threshold,
    "violations": violations,
    "summary": {
        "coverage_percent": round(percent, 2),
        "lines_covered": totals["covered"],
        "lines_total": totals["count"],
    },
}
print(json.dumps(result, indent=2))
sys.exit(0 if not violations else 1)
PYEOF
