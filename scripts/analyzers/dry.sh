#!/usr/bin/env bash
# Analyzer Contract wrapper around jscpd.
# Usage: dry.sh <path>
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

THRESHOLD=3
TARGET="${1:-.}"
OUT_DIR="$(mktemp -d)"
trap 'rm -rf "$OUT_DIR"' EXIT

jscpd "$TARGET" --reporters json --output "$OUT_DIR" --silent \
  --ignore "**/target/**,**/mutants.out*/**,**/build/**,crates/acceptance-tests/tests/**" \
  >/dev/null 2>&1 || true

python3 - "$THRESHOLD" "$OUT_DIR" <<'PYEOF'
import json, sys, os

threshold, out_dir = float(sys.argv[1]), sys.argv[2]
report_path = os.path.join(out_dir, "jscpd-report.json")

if not os.path.exists(report_path):
    print(json.dumps({
        "tool": "jscpd", "metric": "dry", "threshold": threshold,
        "violations": [], "summary": {"note": "no duplication found"},
    }, indent=2))
    sys.exit(0)

with open(report_path) as fh:
    report = json.load(fh)

percent = report["statistics"]["total"]["percentage"]
violations = [
    {
        "first_file": d["firstFile"]["name"],
        "first_lines": f"{d['firstFile']['start']}-{d['firstFile']['end']}",
        "second_file": d["secondFile"]["name"],
        "second_lines": f"{d['secondFile']['start']}-{d['secondFile']['end']}",
        "duplicated_lines": d["lines"],
    }
    for d in report["duplicates"]
]

result = {
    "tool": "jscpd",
    "metric": "dry",
    "threshold": threshold,
    "violations": violations if percent > threshold else [],
    "summary": {
        "duplicated_percent": round(percent, 2),
        "clones": report["statistics"]["total"]["clones"],
    },
}
print(json.dumps(result, indent=2))
sys.exit(0 if percent <= threshold else 1)
PYEOF
