#!/usr/bin/env bash
# Runs the full end-to-end QA suite: one script per qa/*.md procedure, each
# driving the project through its user-facing interface (HTTP endpoint or
# CLI) only. Keep these scripts aligned with qa/*.md; when a procedure file
# changes, update its corresponding script in the same QA work.
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

STATUS=0
for script in "$SCRIPT_DIR"/*.sh; do
  case "$(basename "$script")" in
    run.sh|lib.sh) continue ;;
  esac
  echo "=== $(basename "$script") ===" >&2
  if ! "$script"; then
    STATUS=1
  fi
done

exit $STATUS
