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

# A build cache of this run's own, never the one ordinary builds use.
#
# cargo-mutants tests a *patched copy* of the tree. If CARGO_TARGET_DIR points
# at the normal target/, two different source trees write to one cache, and the
# last mutant's artifacts stay there: a later `cargo test` links a mutated
# library while `git status` is clean, so the suite goes red for a change that
# does not exist. That is silent and extremely confusing, and the constitution's
# own "prefer project-local cache paths" is what leads an agent into it.
#
# Pointing it somewhere dedicated keeps the cache warm between mutation runs --
# which is most of the speed -- while leaving normal builds untouched.
export CARGO_TARGET_DIR="$ROOT_DIR/target-mutants"

# nextest, not `cargo test`. Two reasons, both measured on this project:
# it runs test binaries concurrently rather than one after another
# (trellis-server: 67.7s -> 3.3s; acceptance-tests: 251.8s -> 46.9s), and it
# stops at the first failure -- which for a *caught* mutant is immediately.
#
# The knock-on effect matters as much as the speed. cargo-mutants derives its
# per-mutant timeout as 5x the baseline test run, floored at 20s: a 53s
# `cargo test` baseline meant a hung mutant burned 268 seconds before anyone
# found out. A ~3s nextest baseline puts that at the 20s floor.
TEST_TOOL=(--test-tool nextest)
NEXTEST_PROFILE=(-P mutants)

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
cargo mutants "${SCOPE[@]}" "${FILE_FLAG[@]}" $DIFF_FLAG "${TEST_TOOL[@]}" \
  --no-times "$@" -- "${NEXTEST_PROFILE[@]}"
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

# A timeout is not a kill. It says the suite did not finish, which is a
# statement about the clock, not about the mutant -- the same mutant can time
# out on a loaded machine and be caught on an idle one, and one did exactly
# that during this script's own rework. Counting it as caught inflates the
# kill rate by whatever the machine happened to be doing, and it does so
# silently, in the direction that makes the gate easier to pass.
#
# Timeouts are reported as their own category and as violations, so an
# unfinished run is work to look at rather than a number to trust.
killed = caught
kill_rate = (killed / valid * 100) if valid else 100.0

try:
    with open("mutants.out/missed.txt") as fh:
        survivors = [line.strip() for line in fh if line.strip()]
except FileNotFoundError:
    survivors = []

violations = [{"mutant": s, "outcome": "survived"} for s in survivors]
if timeout:
    violations.append({
        "outcome": "timeout",
        "count": timeout,
        "note": "the suite did not finish; neither caught nor survived. "
                "Re-run these on an idle machine before trusting either answer.",
    })

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
sys.exit(0 if (kill_rate >= threshold and not timeout and mutants_exit in (0,)) else 1)
PYEOF
