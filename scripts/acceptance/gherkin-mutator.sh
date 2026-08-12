#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"
aps_require
exec bb --config "$APS_HOME/bb.edn" gherkin-mutator "$@"
