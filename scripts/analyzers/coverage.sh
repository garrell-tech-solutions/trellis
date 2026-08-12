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
else
  SCOPE=(--workspace)
fi

RAW_FILE="$(mktemp)"
trap 'rm -f "$RAW_FILE"' EXIT
# --include-ignored so property tests (run separately per stack.prompt, but
# tagged #[ignore] to stay out of the default `cargo test` run) still count
# toward coverage instead of reading as a false gap.
cargo llvm-cov "${SCOPE[@]}" --json --summary-only >"$RAW_FILE" 2>/dev/null -- --include-ignored

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
