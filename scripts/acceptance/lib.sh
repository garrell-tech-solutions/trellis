#!/usr/bin/env bash
# Shared helpers for the acceptance pipeline wrapper scripts.
# APS = github.com/unclebob/Acceptance-Pipeline-Specification, installed
# out-of-tree (shared across worktrees/roles) rather than vendored.

: "${APS_HOME:=$HOME/.cache/swarmforge/aps-pipeline}"

aps_require() {
  if [[ ! -f "$APS_HOME/bb.edn" ]]; then
    echo "APS pipeline not found at $APS_HOME" >&2
    echo "Install it: git clone https://github.com/unclebob/Acceptance-Pipeline-Specification.git $APS_HOME" >&2
    exit 1
  fi
}
