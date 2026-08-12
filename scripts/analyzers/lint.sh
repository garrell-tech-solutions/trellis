#!/usr/bin/env bash
# Analyzer Contract wrapper around clippy.
# Usage: lint.sh <path>
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

THRESHOLD=0
TARGET="${1:-.}"

if pkg="$(pkg_for_path "$TARGET" 2>/dev/null)"; then
  SCOPE=(-p "$pkg")
else
  SCOPE=(--workspace)
fi

RAW_FILE="$(mktemp)"
trap 'rm -f "$RAW_FILE"' EXIT
cargo clippy "${SCOPE[@]}" --all-targets --message-format=json -- -D warnings >"$RAW_FILE" 2>/dev/null || true

python3 - "$THRESHOLD" "$RAW_FILE" <<'PYEOF'
import json, sys

threshold, raw_file = float(sys.argv[1]), sys.argv[2]
violations = []
with open(raw_file) as fh:
    lines = fh.readlines()
for line in lines:
    line = line.strip()
    if not line:
        continue
    d = json.loads(line)
    if d.get("reason") != "compiler-message":
        continue
    msg = d.get("message", {})
    if msg.get("level") not in ("warning", "error"):
        continue
    span = next((s for s in msg.get("spans", []) if s.get("is_primary")), None)
    violations.append({
        "level": msg["level"],
        "message": msg["message"],
        "file": span["file_name"] if span else None,
        "line": span["line_start"] if span else None,
    })

result = {
    "tool": "clippy",
    "metric": "lint",
    "threshold": threshold,
    "violations": violations,
    "summary": {"diagnostics": len(violations)},
}
print(json.dumps(result, indent=2))
sys.exit(0 if not violations else 1)
PYEOF
