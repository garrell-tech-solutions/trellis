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
# That default runs the opposite way from the scope ignores below, and both
# directions are chosen rather than inherited. A *format* left out by
# accident can only add noise to a code number, so unknown formats default
# out. A *path* left out by accident is a body of product code nobody is
# measuring, so unknown paths default in. Neither list may be read as the
# precedent for the other.
#
# Rust is the gate's subject. Bash is in deliberately, not by accident: the
# shell that is not a harness -- scripts/analyzers, scripts/ci, ops -- is
# code with no design reason to repeat itself, its duplication is real and
# already has somewhere to go (scripts/analyzers/lib.sh,
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

# Ignores, in two groups, because they answer two different questions.
#
# MECHANICAL -- not source in this tree at all. The worktree entries matter
# even though .gitignore already lists .claude/ and .worktrees/ and jscpd
# honours .gitignore by default: those directories are whole copies of the
# tree, and measured they take this repo from 2.15% to 11.57%. Relying on
# that default leaves the exclusion one --no-gitignore or one .jscpd.json
# away from silently reversing, so it is stated here as well.
#
# .git/ is the same class and was being missed: .gitignore cannot list it
# and jscpd does not skip it, so git's own sample hooks were measured --
# nine of the fourteen are recognised as bash, 644 lines, 8% of all the bash
# this gate counted. None of it is written here, and how much of it there is
# depends on the reader's git version rather than on this repo.
IGNORE_MECHANICAL="**/target/**,**/mutants.out*/**,**/build/**,**/tmp/**"
IGNORE_MECHANICAL="$IGNORE_MECHANICAL,**/.worktrees/**,**/.claude/**,**/.git/**"

# SCOPE -- T-dry-measures-product-code, settled by the owner 2026-08-24:
# this gate measures product code only, and the acceptance harness is
# excluded. The decision named the harness and left the rest of the boundary
# to be drawn; it is drawn here, once, so the number has a stable meaning.
#
# The test that draws it: a percentage gate is loosened by anything that
# grows with the number of features, because a growing denominator absorbs
# duplication that a fixed one would report. This repo has exactly two such
# bodies of code, and both are per-feature scaffolding whose repetition is
# structural rather than careless:
#
#   crates/acceptance-tests/**  one step module per screen. 8,100 lines,
#                               growing 400-800 per slice, and 62% of every
#                               clone this gate found sat inside it, spread
#                               across *pairs* of screen modules rather than
#                               in one extractable shape. Deduplicated three
#                               times without changing that.
#   scripts/qa/**               one script per feature, 24 of them and 5,000
#                               lines, on the same growth law one language
#                               down, and it already has scripts/qa/lib.sh
#                               for the parts that do factor out. Left in it
#                               dilutes: the whole-tree figure reads 0.4
#                               points below the product-Rust figure it is
#                               supposed to be reporting.
#   scripts/acceptance/**       the acceptance harness's own driver. Out
#                               because the decision excludes the acceptance
#                               harness, not because it grows.
#
# Out for a different reason, and stated rather than left to luck: `swarm`
# and swarmforge/ are the pipeline that writes the product, not the product,
# and the scripts are synced from an upstream fork rather than written here.
# Most of them are already invisible because .gitignore lists
# swarmforge/scripts/ -- which is the same one-line-away-from-reversing
# dependency the .worktrees/ and .claude/ entries above refuse to rely on, and
# here it would drop twenty upstream shell scripts into a product number.
#
# Deliberately still measured, because neither grows per feature and both
# have duplication that can actually be removed:
#
#   scripts/analyzers, scripts/ci   gates the product rather than being it,
#                                   but it is fixed-size infrastructure and
#                                   its clones are ordinary copy-paste.
#   ops/*.sh                        installs and runs the shipped binary on
#                                   the owner's machine. If anything in this
#                                   repo is product, deployment is.
#
# And what a path exclusion cannot reach, by design: the product crates keep
# their inline #[cfg(test)] tests in the same files as the code they test,
# and crates/scheduler-core/tests/ holds the properties. Both stay measured.
# "Product code only" is not "no test code" -- it is "not the harness".
#
# crates/acceptance-tests/tests/** used to be listed here on its own; it is
# the generated entrypoint directory and is covered by the crate-wide entry
# above.
IGNORE_SCOPE="crates/acceptance-tests/**,scripts/acceptance/**,scripts/qa/**"
IGNORE_SCOPE="$IGNORE_SCOPE,swarmforge/**,swarm"

# A scope stated as --ignore rather than as an opt-in list of product paths,
# for the reason given against FORMATS: an opt-in list drops a new crate out
# of the gate the day it is added and says nothing. This way round the
# failure is loud -- rename crates/acceptance-tests and the harness lands
# back in the measurement, the percentage jumps, and the build goes red
# asking why.
IGNORE="$IGNORE_MECHANICAL,$IGNORE_SCOPE"

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

python3 - "$THRESHOLD" "$OUT_DIR" "$TARGET" "$FORMATS" "$JSCPD_EXIT" "$IGNORE_SCOPE" <<'PYEOF'
import json, sys, os

threshold, out_dir = float(sys.argv[1]), sys.argv[2]
target, formats, jscpd_exit = sys.argv[3], sys.argv[4], int(sys.argv[5])
scope_excludes = sys.argv[6]

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
# wrapper's. Raised as #52 and deliberately not settled by the scope change
# in #130: one is what the percentage is taken *over*, the other is what it
# counts.
totals = report["statistics"]["total"]
present_formats = report["statistics"].get("formats", {})

# FLOOR: jscpd ran, but did it read anything? `sources` is the file count.
# Pointed at a path that matches nothing it reports a well-formed 0.00%.
# Since #130 this also catches a path that is entirely out of scope --
# dry.sh crates/acceptance-tests measures nothing and must say so rather
# than report a clean harness.
if not totals.get("sources"):
    print(
        f"FAIL: jscpd scanned 0 files under \"{target}\" (formats: {formats}).\n"
        f"      0.00% duplication across nothing measured is not a pass.\n"
        f"      Either the path is wrong, or it lies entirely inside the\n"
        f"      out-of-scope set: {scope_excludes}",
        file=sys.stderr,
    )
    sys.exit(1)

# FLOOR: every format asked for must have resolved to files. A typo in
# FORMATS -- the live risk, since #50 changed that list -- is otherwise
# silent: jscpd skips the unknown name, scans whatever is left, and the
# percentage falls. Spelling `rust` wrong would drop this gate's whole
# subject and still report green. It now also catches a scope exclusion that
# has eaten a whole language: excluding the last measured shell script would
# leave `bash` resolving to nothing, and that is a change to what the gate
# covers, not a quiet improvement in its number.
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
        # scope_excludes is there for the same reason and is the newer half
        # of it: since #130 this number is taken over product code only, and
        # a reader who cannot see what was left out cannot tell a real fall
        # in duplication from a widened exclusion.
        #
        # files_scanned is the same idea one level down, and it is the
        # number the floors above turn into a verdict: a percentage is only
        # as meaningful as the file count it was taken over, so print the
        # count next to it rather than leaving a reader to infer it.
        "files_scanned": totals["sources"],
        "path": target,
        "formats": formats,
        "scope_excludes": scope_excludes,
        "by_format": by_format,
    },
}
print(json.dumps(result, indent=2))
sys.exit(0 if percent <= threshold else 1)
PYEOF
