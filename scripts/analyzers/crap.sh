#!/usr/bin/env bash
# Analyzer Contract wrapper composing coverage (cargo-llvm-cov, per-method
# line-rate via its cobertura export) and complexity (rust-code-analysis-cli)
# into the CRAP score: complexity^2 * (1 - coverage)^3 + complexity.
# Usage: crap.sh <path>
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

THRESHOLD=30
TARGET="${1:-.}"

if pkg="$(pkg_for_path "$TARGET" 2>/dev/null)"; then
  SCOPE=(-p "$pkg")
  REPORT_SCOPE=(-p "$pkg")
else
  SCOPE=(--workspace)
  # `cargo llvm-cov report` has no --workspace: with no package filter it
  # renders everything in the profile, which is what --workspace collected.
  REPORT_SCOPE=()
fi

COBERTURA_FILE="$(mktemp)"
trap 'rm -f "$COBERTURA_FILE"' EXIT

# The instrumented run itself, shared with coverage.sh -- see lib.sh, which
# also carries the reasoning for --include-ignored and --ignore-run-fail. This
# analyzer used to take a second run of the identical suite purely to export
# the same profile in a different format. It is taken here if nothing has taken
# it yet, so crap.sh still works standalone with nothing else having run first.
llvm_cov_profile_ensure "${SCOPE[*]}" "${SCOPE[@]}"

# Rendering only. `report` runs no tests; it exports the profile above. No
# --summary-only here: the per-method line rates are the whole input to CRAP.
cargo llvm-cov report "${REPORT_SCOPE[@]}" --cobertura \
  --output-path "$COBERTURA_FILE" >/dev/null 2>&1

python3 - "$TARGET" "$THRESHOLD" "$SCRIPT_DIR" "$COBERTURA_FILE" <<'PYEOF'
import json, os, sys
import xml.etree.ElementTree as ET

target, threshold, script_dir, cobertura_path = (
    sys.argv[1], float(sys.argv[2]), sys.argv[3], sys.argv[4]
)
sys.path.insert(0, script_dir)
from _common import rust_files_under, production_function_index


def strip_trailing_turbofish(name):
    """Drop a trailing `::<Generic::Path>` instantiation, e.g. turning
    `render_template::<trellis_server::inbox::http::InboxTemplate>` into
    `render_template`. Only a *trailing* bracket is a turbofish; a leading
    one is a trait-impl qualifier (`<Type as Trait<Generic>>::method`) whose
    own method name already survives a plain rsplit, so it's left alone."""
    if not name.endswith(">"):
        return name
    depth = 0
    i = len(name) - 1
    while i >= 0:
        if name[i] == ">":
            depth += 1
        elif name[i] == "<":
            depth -= 1
            if depth == 0:
                break
        i -= 1
    if i > 0 and name[:i].endswith("::"):
        return name[: i - 2]
    return name

# file -> [(short_name, line_rate, [line numbers])]
# Keyed on normpath: cargo-llvm-cov's cobertura export writes filenames
# without a "./" prefix (e.g. "crates/foo/src/lib.rs"), while `find .`
# (rust_files_under) yields paths with one (e.g. "./crates/foo/src/lib.rs").
# Without normalizing both sides to the same form, every lookup below misses
# and every function is silently scored at 0% coverage.
coverage_by_file = {}
tree = ET.parse(cobertura_path)
for cls in tree.iter("class"):
    filename = os.path.normpath(cls.get("filename"))
    methods_el = cls.find("methods")
    if methods_el is None:
        continue
    entries = coverage_by_file.setdefault(filename, [])
    for m in methods_el.findall("method"):
        full_name = m.get("name")
        short_name = strip_trailing_turbofish(full_name).rsplit("::", 1)[-1]
        if short_name.startswith("{closure"):
            continue
        lines_el = m.find("lines")
        line_numbers = [int(l.get("number")) for l in lines_el.findall("line")] if lines_el is not None else []
        entries.append((short_name, float(m.get("line-rate")), line_numbers))


def coverage_for(filename, fn_name, start, end):
    # A function compiled into several acceptance-test binaries (this crate
    # is linked once per generated *_acceptance.rs) gets one cobertura
    # <method> entry per binary. Take the best (max) line-rate seen: the
    # function is covered if any binary that links it exercises it.
    best = None
    for short_name, line_rate, line_numbers in coverage_by_file.get(os.path.normpath(filename), []):
        if short_name != fn_name:
            continue
        if not any(start <= n <= end for n in line_numbers):
            continue
        if best is None or line_rate > best:
            best = line_rate
    return best if best is not None else 0.0


files = rust_files_under(target, ".")

# See complexity.sh: an empty walk is a vacuous pass, not a clean tree.
if not files:
    print(json.dumps({
        "tool": "crap.sh (cargo-llvm-cov + rust-code-analysis-cli)",
        "metric": "crap",
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

# One rust-code-analysis-cli walk, shared with complexity.sh -- see _common.py.
for f, functions in production_function_index(files).items():
    for name, start, end, cyclomatic in functions:
        total_functions += 1
        coverage = coverage_for(f, name, start, end)
        crap = cyclomatic ** 2 * (1 - coverage) ** 3 + cyclomatic
        if crap >= threshold:
            violations.append({
                "file": f,
                "function": name,
                "line": start,
                "cyclomatic_complexity": cyclomatic,
                "coverage": round(coverage, 4),
                "crap_score": round(crap, 2),
            })

result = {
    "tool": "crap.sh (cargo-llvm-cov + rust-code-analysis-cli)",
    "metric": "crap",
    "threshold": threshold,
    "violations": violations,
    "summary": {"functions_analyzed": total_functions},
}
print(json.dumps(result, indent=2))
sys.exit(0 if not violations else 1)
PYEOF
