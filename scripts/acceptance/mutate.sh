#!/usr/bin/env bash
# Gherkin acceptance mutation for every feature file (hardener role: "run
# soft Gherkin acceptance mutation" as part of the handoff verification
# sequence). Reuses the generated tests/metadata/IR already produced by
# run.sh, and drives them through the project's runner adapter.
#
# Usage: mutate.sh [--level full|hard|soft] [feature-slug ...]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"
aps_require

LEVEL="hard"
if [[ "${1:-}" == "--level" ]]; then
  LEVEL="$2"
  shift 2
fi

if [[ $# -gt 0 ]]; then
  SLUGS=("$@")
else
  SLUGS=()
  for feature in features/*.feature; do
    SLUGS+=("$(basename "$feature" .feature)")
  done
fi

STATUS=0
for slug in "${SLUGS[@]}"; do
  echo "=== mutating $slug (level=$LEVEL) ===" >&2
  bb --config "$APS_HOME/bb.edn" gherkin-mutator \
    --feature "features/${slug}.feature" \
    --work-dir "build/acceptance-mutation/${slug}" \
    --generated-dir crates/acceptance-tests/tests \
    --workers 1 \
    --status-interval 15s \
    --level "$LEVEL" \
    --runner-worker "bb scripts/acceptance/mutation-runner-adapter.bb ${slug}" \
    || STATUS=1
done

exit $STATUS
