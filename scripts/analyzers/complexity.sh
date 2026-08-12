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
import json, subprocess, sys

target, threshold, script_dir = sys.argv[1], float(sys.argv[2]), sys.argv[3]
sys.path.insert(0, script_dir)
from _common import rust_files_under, production_functions

files = rust_files_under(target, ".")
violations = []
total_functions = 0

for f in files:
    raw = subprocess.run(
        ["rust-code-analysis-cli", "-p", f, "-m", "-O", "json"],
        capture_output=True, text=True, check=True,
    ).stdout
    with open(f) as fh:
        source = fh.read()
    for line in raw.splitlines():
        if not line.strip():
            continue
        unit = json.loads(line)
        for name, start, end, cyclomatic in production_functions(unit, source):
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
