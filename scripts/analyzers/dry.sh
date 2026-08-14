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
IGNORE="**/target/**,**/mutants.out*/**,**/build/**"
IGNORE="$IGNORE,crates/acceptance-tests/tests/**,**/tmp/**"
IGNORE="$IGNORE,**/.worktrees/**,**/.claude/**"

jscpd "$TARGET" --reporters json --output "$OUT_DIR" --silent \
  --format "$FORMATS" --ignore "$IGNORE" \
  >/dev/null 2>&1 || true

python3 - "$THRESHOLD" "$OUT_DIR" "$TARGET" "$FORMATS" <<'PYEOF'
import json, sys, os

threshold, out_dir = float(sys.argv[1]), sys.argv[2]
target, formats = sys.argv[3], sys.argv[4]

report_path = os.path.join(out_dir, "jscpd-report.json")

if not os.path.exists(report_path):
    print(json.dumps({
        "tool": "jscpd", "metric": "dry", "threshold": threshold,
        "violations": [],
        "summary": {
            "note": "no duplication found",
            "path": target, "formats": formats,
        },
    }, indent=2))
    sys.exit(0)

with open(report_path) as fh:
    report = json.load(fh)

# jscpd's "percentage" is duplicated *lines*; "percentageTokens" is tokens.
# Every DRY figure this project has recorded is the line number, so that is
# what is read here. stack.prompt calls the threshold "max % duplicated
# tokens", which does not describe this; switching the reading would
# redefine what 3 means and is the constitution owner's call, not this
# wrapper's.
percent = report["statistics"]["total"]["percentage"]
by_format = {
    name: {
        "clones": stats["clones"],
        "duplicated_percent": round(stats["percentage"], 2),
        "duplicated_lines": stats["duplicatedLines"],
        "lines": stats["lines"],
    }
    for name, stats in sorted(report["statistics"].get("formats", {}).items())
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
        "clones": report["statistics"]["total"]["clones"],
        # A figure nobody can reproduce is not a measurement: record the
        # path and format set that produced this one, and attribute the
        # clones per language so a move in the number can be read without
        # re-deriving it by hand.
        "path": target,
        "formats": formats,
        "by_format": by_format,
    },
}
print(json.dumps(result, indent=2))
sys.exit(0 if percent <= threshold else 1)
PYEOF
