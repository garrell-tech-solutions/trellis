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

# Formats. This gate exists to catch copy-pasted code, and jscpd left
# unrestricted measures whatever it recognises -- markdown, gherkin, sql,
# toml, yaml, html -- so prose and declarative files move a code number
# (#50). The restriction is by --format rather than by --ignore on purpose:
# a documentation directory added tomorrow is out by default instead of by
# someone remembering to list it.
#
# Rust is the gate's subject. Bash is in deliberately, not by accident:
# scripts/**/*.sh is code with no design reason to repeat itself, its
# duplication is real and already has somewhere to go (scripts/qa/lib.sh,
# scripts/analyzers/_common.py), and including it costs no headroom.
# Everything else is out because its repetition is either intended
# (features/*.feature restate Given/When/Then by design; docs/decisions.md
# restates rulings on purpose) or forbidden to remove (migrations are
# append-only under T-migrations-append-only, enforced next door by
# scripts/ci/migration_immutability.sh). A gate that reports a defect
# nobody is allowed to fix teaches its readers to ignore it.
#
# The reasoning is T-complexity-8's: a metric guards the architecture, it
# does not shape it. Left scanning prose, this one eventually gets "fixed"
# by deduplicating documentation.
FORMATS="rust,bash"

# Ignores. The worktree entries matter even though .gitignore already lists
# .claude/ and .worktrees/ and jscpd honours .gitignore by default: those
# directories are whole copies of the tree, and measured they take this
# repo from 2.15% to 11.57%. Relying on that default leaves the exclusion
# one --no-gitignore or one .jscpd.json away from silently reversing, so it
# is stated here as well.
#
# crates/acceptance-tests/** (the whole crate, not just tests/) is excluded
# as of stack.prompt's 2026-08-24 scope change: the gate exists to catch
# copy-pasted product logic, and 62% of what it was flagging was duplication
# *between* per-screen step modules under src/steps/ -- structural to a
# per-feature APS harness (each screen earns its own module) and not
# extractable, the same "reports a defect nobody is allowed to fix" problem
# the format restriction above already exists to avoid. Product-only
# measured 2.09% against 3.54% including the harness at the time, so the
# threshold itself did not need to move.
IGNORE="**/target/**,**/mutants.out*/**,**/build/**"
IGNORE="$IGNORE,crates/acceptance-tests/**,**/tmp/**"
IGNORE="$IGNORE,**/.worktrees/**,**/.claude/**"

# jscpd's status is captured rather than propagated, but it is not discarded:
# it is handed to the reader below, which fails on it. The same shape
# scripts/ci/complexity_baseline.sh uses, and for the same reason -- what
# matters is whether the tool produced a report that actually measured
# something, and that question is answered where the report is parsed.
set +e
jscpd "$TARGET" --reporters json --output "$OUT_DIR" --silent \
  --format "$FORMATS" --ignore "$IGNORE" \
  >/dev/null 2>&1
JSCPD_EXIT=$?
set -e

python3 - "$THRESHOLD" "$OUT_DIR" "$TARGET" "$FORMATS" "$JSCPD_EXIT" <<'PYEOF'
import json, sys, os

threshold, out_dir = float(sys.argv[1]), sys.argv[2]
target, formats, jscpd_exit = sys.argv[3], sys.argv[4], int(sys.argv[5])

report_path = os.path.join(out_dir, "jscpd-report.json")

# FLOOR. A missing report is a broken run, not a clean one.
#
# This branch used to print "no duplication found" and exit 0, which made
# every way jscpd can fail to run -- not installed, killed, a bad --format,
# an unreadable path -- indistinguishable from a tree with no duplication in
# it. The gate reported 0.00%, which reads as an improvement rather than as
# an absent measurement, and nothing anywhere said the tool had not run.
#
# The floors here are the ones acceptance_entrypoints.sh and
# complexity_baseline.sh already apply next door: a gate that measured
# nothing must fail, never pass quietly.
if not os.path.exists(report_path):
    print(
        f"FAIL: jscpd produced no report at {report_path} (exit {jscpd_exit}).\n"
        f"      Nothing was measured, so there is no DRY verdict to give.\n"
        f"      Is jscpd installed?  npm install -g jscpd",
        file=sys.stderr,
    )
    sys.exit(1)

try:
    with open(report_path) as fh:
        report = json.load(fh)
except (json.JSONDecodeError, OSError) as exc:
    print(
        f"FAIL: jscpd's report is unreadable (exit {jscpd_exit}): {exc}",
        file=sys.stderr,
    )
    sys.exit(1)

# jscpd's "percentage" is duplicated *lines*; "percentageTokens" is tokens.
# Every DRY figure this project has recorded is the line number, so that is
# what is read here. stack.prompt calls the threshold "max % duplicated
# tokens", which does not describe this; switching the reading would
# redefine what 3 means and is the constitution owner's call, not this
# wrapper's.
totals = report["statistics"]["total"]
present_formats = report["statistics"].get("formats", {})

# FLOOR: jscpd ran, but did it read anything? `sources` is the file count.
# Pointed at a path that matches nothing it reports a well-formed 0.00%.
if not totals.get("sources"):
    print(
        f"FAIL: jscpd scanned 0 files under \"{target}\" (formats: {formats}).\n"
        f"      0.00% duplication across nothing measured is not a pass.\n"
        f"      Either the path is wrong or the ignore list excludes the tree.",
        file=sys.stderr,
    )
    sys.exit(1)

# FLOOR: every format asked for must have resolved to files. A typo in
# FORMATS -- the live risk, since #50 changed that list -- is otherwise
# silent: jscpd skips the unknown name, scans whatever is left, and the
# percentage falls. Spelling `rust` wrong would drop this gate's whole
# subject and still report green.
missing_formats = [f for f in formats.split(",") if f and f not in present_formats]
if missing_formats:
    print(
        f"FAIL: these formats matched no files: {', '.join(missing_formats)}\n"
        f"      requested: {formats}\n"
        f"      resolved:  {', '.join(sorted(present_formats)) or '(none)'}\n"
        f"      Either FORMATS in this script has a typo, or the format list\n"
        f"      has gone stale against the tree. Both silently shrink what\n"
        f"      this gate measures, so neither may pass.",
        file=sys.stderr,
    )
    sys.exit(1)

percent = totals["percentage"]
by_format = {
    name: {
        "clones": stats["clones"],
        "duplicated_percent": round(stats["percentage"], 2),
        "duplicated_lines": stats["duplicatedLines"],
        "lines": stats["lines"],
    }
    for name, stats in sorted(present_formats.items())
}
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
        "clones": totals["clones"],
        # A figure nobody can reproduce is not a measurement: record the
        # path and format set that produced this one, and attribute the
        # clones per language so a move in the number can be read without
        # re-deriving it by hand.
        #
        # files_scanned is the same idea one level down, and it is the
        # number the floors above turn into a verdict: a percentage is only
        # as meaningful as the file count it was taken over, so print the
        # count next to it rather than leaving a reader to infer it.
        "files_scanned": totals["sources"],
        "path": target,
        "formats": formats,
        "by_format": by_format,
    },
}
print(json.dumps(result, indent=2))
sys.exit(0 if percent <= threshold else 1)
PYEOF
