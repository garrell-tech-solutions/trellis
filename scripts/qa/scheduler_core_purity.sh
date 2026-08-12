#!/usr/bin/env bash
# Executable QA procedure: qa/scheduler_core_purity.md (covers
# features/scheduler_core_purity.feature). This is a release-gate check
# (decisions.md T4).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

TREE="$(cargo tree -p scheduler-core)"

FAILURES=0
for forbidden in tokio sqlx; do
  if echo "$TREE" | sed -E 's/^[^a-zA-Z0-9_]*//' | awk '{print $1}' | grep -qx "$forbidden"; then
    echo "FAIL: scheduler-core dependency tree contains forbidden dependency \"$forbidden\"" >&2
    FAILURES=1
  fi
done

if [[ "$FAILURES" -ne 0 ]]; then
  echo "$TREE" >&2
  exit 1
fi
echo "PASS: scheduler_core_purity"
