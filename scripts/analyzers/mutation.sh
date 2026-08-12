#!/usr/bin/env bash
# Analyzer Contract wrapper around cargo-mutants (differential by default,
# per swarmforge/roles/hardener.prompt: "Always use differential mutation
# against the manifest unless explicitly directed otherwise" -- cargo-mutants'
# manifest is its own mutants.out/ state, reused via --iterate).
# Usage: mutation.sh <path> [extra cargo-mutants args...]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

THRESHOLD=90
TARGET="${1:?usage: mutation.sh <path> [extra cargo-mutants args...]}"
shift || true

DIFF_FLAG="--iterate"
if [[ "${HARDENER_MUTATION_LEVEL:-}" == "full" ]]; then
  DIFF_FLAG=""
fi

if pkg="$(pkg_for_path "$TARGET" 2>/dev/null)"; then
  SCOPE=(-p "$pkg")
else
  SCOPE=(--workspace)
fi

FILE_FLAG=()
if [[ -f "$TARGET" ]]; then
  REL="$(python3 -c "import os,sys; print(os.path.relpath(sys.argv[1], sys.argv[2]))" "$TARGET" "$ROOT_DIR")"
  FILE_FLAG=(-f "$REL")
fi

set +e
cargo mutants "${SCOPE[@]}" "${FILE_FLAG[@]}" $DIFF_FLAG --no-times "$@"
MUTANTS_EXIT=$?
set -e

python3 - "$THRESHOLD" "$MUTANTS_EXIT" <<'PYEOF'
import json, sys

threshold, mutants_exit = float(sys.argv[1]), int(sys.argv[2])

try:
    with open("mutants.out/outcomes.json") as fh:
        outcomes = json.load(fh)
except FileNotFoundError:
    if mutants_exit == 0:
        # No mutants were in scope (e.g. a file with no mutable code).
        print(json.dumps({
            "tool": "cargo-mutants", "metric": "mutation", "threshold": threshold,
            "violations": [], "summary": {"note": "no mutants in scope"},
        }, indent=2))
        sys.exit(0)
    print(json.dumps({
        "tool": "cargo-mutants", "metric": "mutation", "threshold": threshold,
        "violations": [{"error": f"cargo-mutants exited {mutants_exit} with no outcomes.json"}],
        "summary": {"note": "cargo-mutants failed to run"},
    }, indent=2))
    sys.exit(1)

total = outcomes["total_mutants"]
unviable = outcomes["unviable"]
caught = outcomes["caught"]
timeout = outcomes["timeout"]
missed = outcomes["missed"]
valid = total - unviable
killed = caught + timeout
kill_rate = (killed / valid * 100) if valid else 100.0

try:
    with open("mutants.out/missed.txt") as fh:
        survivors = [line.strip() for line in fh if line.strip()]
except FileNotFoundError:
    survivors = []

violations = [{"mutant": s} for s in survivors]

result = {
    "tool": "cargo-mutants",
    "metric": "mutation",
    "threshold": threshold,
    "violations": violations,
    "summary": {
        "kill_rate_percent": round(kill_rate, 2),
        "total_mutants": total,
        "caught": caught,
        "missed": missed,
        "timeout": timeout,
        "unviable": unviable,
    },
}
print(json.dumps(result, indent=2))
sys.exit(0 if (kill_rate >= threshold and mutants_exit in (0,)) else 1)
PYEOF
