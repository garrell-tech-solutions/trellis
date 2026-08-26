#!/usr/bin/env bash
# CI gate for T-one-front-door-per-capability: no capability reaches into
# another capability's `store`.
#
# WHAT IT ENFORCES, AND WHY THAT LINE
#
#   `crates/trellis-server/src/<capability>/` is a business-domain package
#   (T-package-by-business-domain). Each one owns its tables through its own
#   `store` submodule and exposes what other capabilities may ask for as
#   named functions on its `mod.rs` -- its front door. `triage` asks
#   `quota::name_standing`; it does not `SELECT` from `quotas`.
#
#   The rule is scoped to `store` rather than to every submodule, and that is
#   deliberate. `store` is the persistence port: the company architecture
#   standard at https://www.garrellts.com/docs/agent-architecture forbids
#   leaking storage concepts back across it, and a caller in another
#   capability naming `<other>::store::` has necessarily done so -- it now
#   depends on that capability's row shapes and query surface, which is
#   exactly the coupling the package boundary exists to prevent. `view` is
#   not covered: `inbox::view::CaptureRow` is a shared *view model*,
#   deliberately consumed by `capture::http`, and a rule that called that a
#   violation would be a rule about file names rather than about direction.
#
#   Test code is exempt. Every capability's unit tests build fixtures through
#   `triage::store::insert_task` and friends, and that is a test arranging a
#   database row, not production policy taking a dependency. Exempting it is
#   what keeps the rule about the shipped dependency graph; without the
#   exemption the gate would report twenty-four fixtures and nothing else.
#
# WHY IT IS NOT A qa/*.md PROCEDURE
#
#   Same reasoning as decision_slugs.sh and migration_immutability.sh next
#   door: it asserts a property of the repository, not of the running
#   program, so it lives under scripts/ci/ and scripts/qa/run.sh does not
#   pick it up.
#
# HOW IT KNOWS WHERE THE TESTS START
#
#   By the project's own layout: `#[cfg(test)] mod tests` is the last item in
#   a capability source file, so everything from the first `#[cfg(test)]`
#   line onward is test code. That is a convention, not a language rule, so
#   the script checks it rather than trusting it -- a file with a second
#   `#[cfg(test)]` fails the gate by name, and the heuristic cannot rot into
#   silently skipping production code.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

SRC="crates/trellis-server/src"
FAILURES=0

for file in $(find "$SRC" -mindepth 2 -name '*.rs' | sort); do
  capability="$(basename "$(dirname "$file")")"
  [[ "$capability" == "platform" ]] && continue

  markers="$(grep -c '^#\[cfg(test)\]' "$file" || true)"
  if [[ "$markers" -gt 1 ]]; then
    echo "FAIL: $file has $markers \`#[cfg(test)]\` items; this gate assumes at" >&2
    echo "      most one, as the file's last item. Split the file or teach the" >&2
    echo "      gate to track brace depth -- do not leave it guessing." >&2
    FAILURES=1
    continue
  fi

  if [[ "$markers" -eq 1 ]]; then
    last_production_line="$(( $(grep -n '^#\[cfg(test)\]' "$file" | cut -d: -f1) - 1 ))"
  else
    last_production_line="$(wc -l < "$file")"
  fi
  [[ "$last_production_line" -lt 1 ]] && continue

  while IFS=: read -r line_no text; do
    other="$(echo "$text" | grep -oE 'crate::[a-z_]+::store' | head -1 | cut -d: -f3)"
    [[ -z "$other" || "$other" == "$capability" ]] && continue
    echo "FAIL: $file:$line_no reaches into \`$other\`'s store:" >&2
    echo "      ${text# }" >&2
    echo "      Ask \`crate::$other\` for what you need; do not read its table." >&2
    FAILURES=1
  done < <(head -n "$last_production_line" "$file" \
    | grep -nE 'crate::[a-z_]+::store' \
    | grep -vE '^[0-9]+:\s*(///|//!|//)')
done

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: capability_front_doors"
