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
#
# --ignore-run-fail because this run measures coverage; it does not adjudicate
# pass or fail. Coverage instrumentation makes the binary several times slower,
# and features/capture_endpoint.feature asserts a 50ms response budget --
# measured on this machine at 746ms under instrumentation against 0.11s for the
# whole uninstrumented feature. That assertion is about the product's speed, and
# the instrumented build is not the product. Without this flag the analyzer
# fails intermittently with a timing error that says nothing about coverage:
# three consecutive runs on 2026-08-14 gave 101, 0, 0.
#
# It masks nothing. The generated entrypoints run every scenario to completion
# and collect the outcomes before asserting (see scripts/acceptance/generate.bb),
# so a failed assertion does not cut the walk short and the profile is complete
# either way. And the verdict on whether the tests pass is taken twice already,
# by `cargo test --workspace` in CI's `gate` job and by
# scripts/acceptance/run.sh, which CI runs -- uninstrumented, at honest speed --
# before this. A build failure still fails here, because that is not a run
# failure.
cargo llvm-cov "${SCOPE[@]}" --json --summary-only --ignore-run-fail \
  >"$RAW_FILE" 2>/dev/null -- --include-ignored

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
