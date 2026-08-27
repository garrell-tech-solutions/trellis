#!/usr/bin/env bash
# Analyzer Contract wrapper around rust-code-analysis-cli (cyclomatic
# complexity, production functions only -- see _common.py).
# Usage: complexity.sh <path>
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

THRESHOLD=8
TARGET="${1:-.}"

python3 - "$TARGET" "$THRESHOLD" "$SCRIPT_DIR" <<'PYEOF'
import json, sys

target, threshold, script_dir = sys.argv[1], float(sys.argv[2]), sys.argv[3]
sys.path.insert(0, script_dir)
from _common import rust_files_under, production_function_index

files = rust_files_under(target, ".")

# A walk that returns nothing must fail, not pass. Same floor as
# trellis-server's platform/boundary.rs, and for the same reason: an analyzer
# with no input exits 0 and reports "no violations", which is indistinguishable
# from a clean tree on a CI summary line. Mistype the path, point the gate at a
# directory that has been renamed away, or land an exclusion that swallows the
# whole tree, and the gate silently stops covering anything -- which is the
# exact defect class issue #26 exists to close.
if not files:
    print(json.dumps({
        "tool": "rust-code-analysis-cli",
        "metric": "complexity",
        "threshold": threshold,
        "violations": [{
            "error": f"no Rust source files found under {target!r}",
            "hint": "nothing was analyzed, so this is a vacuous pass, not a clean tree",
        }],
        "summary": {"functions_analyzed": 0},
    }, indent=2))
    sys.exit(1)

violations = []
total_functions = 0

# One rust-code-analysis-cli walk, shared with crap.sh -- see _common.py.
for f, functions in production_function_index(files).items():
    for name, start, end, cyclomatic in functions:
        total_functions += 1
        if cyclomatic > threshold:
            violations.append({
                "file": f,
                "function": name,
                "line": start,
                "cyclomatic_complexity": cyclomatic,
            })

result = {
    "tool": "rust-code-analysis-cli",
    "metric": "complexity",
    "threshold": threshold,
    "violations": violations,
    "summary": {"functions_analyzed": total_functions},
}
print(json.dumps(result, indent=2))
sys.exit(0 if not violations else 1)
PYEOF
