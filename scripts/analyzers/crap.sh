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
else
  SCOPE=(--workspace)
fi

COBERTURA_FILE="$(mktemp)"
trap 'rm -f "$COBERTURA_FILE"' EXIT
# --include-ignored: see coverage.sh.
cargo llvm-cov "${SCOPE[@]}" --cobertura --output-path "$COBERTURA_FILE" >/dev/null 2>&1 -- --include-ignored

python3 - "$TARGET" "$THRESHOLD" "$SCRIPT_DIR" "$COBERTURA_FILE" <<'PYEOF'
import json, subprocess, sys
import xml.etree.ElementTree as ET

target, threshold, script_dir, cobertura_path = (
    sys.argv[1], float(sys.argv[2]), sys.argv[3], sys.argv[4]
)
sys.path.insert(0, script_dir)
from _common import rust_files_under, production_functions

# file -> [(short_name, line_rate, [line numbers])]
coverage_by_file = {}
tree = ET.parse(cobertura_path)
for cls in tree.iter("class"):
    filename = cls.get("filename")
    methods_el = cls.find("methods")
    if methods_el is None:
        continue
    entries = coverage_by_file.setdefault(filename, [])
    for m in methods_el.findall("method"):
        full_name = m.get("name")
        short_name = full_name.rsplit("::", 1)[-1]
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
    for short_name, line_rate, line_numbers in coverage_by_file.get(filename, []):
        if short_name != fn_name:
            continue
        if not any(start <= n <= end for n in line_numbers):
            continue
        if best is None or line_rate > best:
            best = line_rate
    return best if best is not None else 0.0


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
