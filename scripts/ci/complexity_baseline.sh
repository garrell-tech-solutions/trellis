#!/usr/bin/env bash
# CI gate for T-complexity-8 (issue #26, trap 1): the set of functions over the
# complexity threshold must be exactly the set recorded in
# scripts/ci/complexity-baseline.json.
#
# WHY THIS IS NOT JUST `complexity.sh || exit 1`
#
#   Because that build is red today and would stay red forever. The analyzer
#   reports five violations, all Gherkin step dispatchers, and the project has
#   already refused the only change that would move them: a (Regex, handler)
#   table needing roughly forty boxed-future wrappers to replace forty one-line
#   branches. docs/decisions-history.md, 2026-08-12, in bold: "Do not flatten these
#   into a registry to make the number go down."
#
#   A gate that can only be satisfied by a change the project forbids is not a
#   gate. It is a red X that every reviewer learns to scroll past, and once
#   they do, the sixth violation arrives unnoticed -- which is the failure this
#   whole issue is about, reproduced by the fix for it.
#
#   The alternative is not to raise the threshold. Raising it to 17 would
#   silently permit a genuine branching thicket anywhere in the workspace, and
#   the threshold is set in swarmforge/constitution/articles/stack.prompt,
#   which no agent may edit. The threshold stays at 8, complexity.sh keeps
#   reporting all five and keeps exiting non-zero (the Analyzer Contract says
#   it must), and this script -- not the analyzer -- decides what is news.
#
# WHAT IT ACTUALLY GUARDS
#
#   The property worth enforcing is not "the number is zero", which is false
#   and will stay false. It is "the known-red set does not grow, and does not
#   change shape without someone saying so". So:
#
#     - a violation absent from the baseline fails      (a sixth dispatcher, or
#                                                        a real thicket)
#     - a baselined score that moved fails, either way  (logic creeping into a
#                                                        dispatch chain)
#     - a baseline entry with no matching violation fails (the row is stale and
#                                                        now covers nothing)
#
#   The scores are exact rather than ceilings on purpose. This project has
#   already been bitten by movement inside a stable total: the 2026-08-12 entry
#   recorded "three violations remain" while steps/mod.rs::dispatch went 5 -> 9
#   in that same slice, the count holding at three only because a different
#   function dropped off. Under exact matching that commit does not merge until
#   the number is written down. Every subsequent addition -- inbox_view,
#   triage_from_page -- was legitimate and would cost one reviewed line in the
#   baseline file, which is the point: the addition becomes visible in a diff
#   instead of being discovered later by re-measuring.
#
#   Failing on a *stale* row matters as much as failing on a new one. A
#   baseline that keeps naming a function which has been renamed or fixed is an
#   exception list quietly covering nothing, and an exception list is the one
#   place a real violation can hide -- the same reasoning
#   trellis-server's platform/boundary.rs used when it refused one.
#
# Usage: scripts/ci/complexity_baseline.sh [path]
#        path defaults to "." (the whole tree; nested worktrees are excluded by
#        scripts/analyzers/_common.py).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

TARGET="${1:-.}"
BASELINE="scripts/ci/complexity-baseline.json"
ANALYZER="scripts/analyzers/complexity.sh"

if [[ ! -f "$BASELINE" ]]; then
  echo "FAIL: $BASELINE not found -- nothing to compare against." >&2
  exit 1
fi

REPORT="$(mktemp)"
trap 'rm -f "$REPORT"' EXIT

# complexity.sh exits non-zero whenever it finds a violation, which is the
# normal case here, so its status is captured rather than propagated. What
# matters is that it produced a parseable report at all: a missing
# rust-code-analysis-cli also exits non-zero, and treating that as "no
# violations" would be the vacuous pass this gate exists to prevent.
set +e
"$ANALYZER" "$TARGET" >"$REPORT" 2>/dev/null
ANALYZER_EXIT=$?
set -e

python3 - "$BASELINE" "$REPORT" "$ANALYZER_EXIT" "$TARGET" <<'PYEOF'
import json, sys

baseline_path, report_path, analyzer_exit, target = (
    sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
)

try:
    with open(report_path) as fh:
        report = json.load(fh)
except (json.JSONDecodeError, OSError) as exc:
    print(
        f"FAIL: scripts/analyzers/complexity.sh produced no readable report "
        f"(exit {analyzer_exit}): {exc}\n"
        f"      Is rust-code-analysis-cli installed?\n"
        f"        cargo install rust-code-analysis-cli",
        file=sys.stderr,
    )
    sys.exit(1)

with open(baseline_path) as fh:
    baseline_doc = json.load(fh)

analyzed = report.get("summary", {}).get("functions_analyzed", 0)
if not analyzed:
    print(
        f"FAIL: the analyzer examined 0 functions under {target!r}.\n"
        f"      Nothing was measured, so any verdict here would be vacuous.",
        file=sys.stderr,
    )
    sys.exit(1)

# complexity.sh reports "./crates/..." for target "." and "crates/..." for
# target "crates". Normalize so the baseline reads the same either way.
def norm(path):
    return path[2:] if path.startswith("./") else path


current = {}
malformed = []
for v in report.get("violations", []):
    if "file" not in v or "function" not in v:
        malformed.append(v)
        continue
    current[(norm(v["file"]), v["function"])] = v["cyclomatic_complexity"]

if malformed:
    print("FAIL: the analyzer reported a violation this gate cannot key:", file=sys.stderr)
    for v in malformed:
        print(f"  {json.dumps(v)}", file=sys.stderr)
    print(
        "\n  complexity.sh emits an entry of this shape when it could not walk\n"
        "  the tree at all. That is a broken run, not a clean one.",
        file=sys.stderr,
    )
    sys.exit(1)

accepted = {
    (norm(e["file"]), e["function"]): e["cyclomatic_complexity"]
    for e in baseline_doc["accepted"]
}

threshold = report.get("threshold")
baseline_threshold = baseline_doc.get("threshold")
if baseline_threshold is not None and float(baseline_threshold) != float(threshold):
    print(
        f"FAIL: {baseline_path} records threshold {baseline_threshold}, but "
        f"the analyzer used {threshold}.\n"
        f"      The threshold lives in "
        f"swarmforge/constitution/articles/stack.prompt and is not an agent's\n"
        f"      to change. If it moved deliberately, every score below was "
        f"measured against the old one.",
        file=sys.stderr,
    )
    sys.exit(1)

new = sorted(k for k in current if k not in accepted)
stale = sorted(k for k in accepted if k not in current)
moved = sorted(
    (k, accepted[k], current[k])
    for k in current
    if k in accepted and current[k] != accepted[k]
)

if not new and not stale and not moved:
    print(
        f"PASS: complexity_baseline ({len(current)} function(s) over the "
        f"threshold of {threshold:g}, exactly the set recorded in "
        f"{baseline_path}; {analyzed} functions analyzed under {target!r})"
    )
    sys.exit(0)

out = [f"FAIL: the over-threshold set does not match {baseline_path}.", ""]

if new:
    out.append("  Over the threshold and NOT in the baseline:")
    for f, fn in new:
        out.append(f"    {f}::{fn}  cyclomatic {current[(f, fn)]:g}  (threshold {threshold:g})")
    out.append("")

if moved:
    out.append("  In the baseline, but the score changed:")
    for (f, fn), was, now in moved:
        direction = "up" if now > was else "down"
        out.append(f"    {f}::{fn}  {was:g} -> {now:g}  ({direction})")
    out.append("")

if stale:
    out.append("  In the baseline, but no longer over the threshold (or gone):")
    for f, fn in stale:
        out.append(f"    {f}::{fn}  recorded at {accepted[(f, fn)]:g}")
    out.append("")

out.append(f"""WHY THIS IS BLOCKED

  T-complexity-8 caps cyclomatic complexity at {threshold:g}. The functions listed in
  {baseline_path} are over that cap deliberately: they are
  Gherkin step dispatchers, regex chains whose every arm is a one-line
  delegation, and T-complexity-8's own rule -- "a function over 8 is carrying
  logic that is not the match; extract that, do not flatten the match" -- says
  there is nothing in them to extract.

  That accepted set is what this gate pins. It does not care that the number is
  above the threshold; it cares that the set is exactly what somebody wrote
  down and defended. Growth, movement and rot are all news.

WHAT TO DO

  - A new function over the threshold, and it is NOT a pure dispatch chain?
    Fix the code, not the baseline. Extract the logic that is not the match.
    This is the case the threshold exists for.

  - A new step-dispatch chain (a sixth step module, say)? Add a row to
    {baseline_path} with its exact score and a one-line
    reason, in the same commit. The reviewer of that diff is the check.

  - A score moved? Say why in the commit and update the number. A dispatch
    chain gaining branches is a new step; a dispatch chain gaining
    *conditionals* is logic sneaking into the match, which is the thing
    T-complexity-8 is actually about.

  - A baseline row now reports as stale? Delete it. A row that matches nothing
    is an exception covering nothing, and an exception list is where a real
    violation hides.

  - Tempted to flatten a dispatcher into a (Regex, handler) table to make the
    number go down? Do not. docs/decisions-history.md, 2026-08-12, settles it: the
    handlers are async with differing arities, so a uniform table needs roughly
    forty boxed-future wrappers to replace forty one-line branches -- exactly
    the "indirection that is strictly worse to read" that T-complexity-8
    refuses. Raising the threshold is not available either: it is set in
    swarmforge/constitution/articles/stack.prompt, which no agent may edit.""")

print("\n".join(out), file=sys.stderr)
sys.exit(1)
PYEOF
