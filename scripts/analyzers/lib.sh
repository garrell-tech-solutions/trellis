#!/usr/bin/env bash
# Shared helpers for scripts/analyzers/*.sh.
#
# Every analyzer accepts a path (file or directory) and resolves it to the
# owning cargo package by walking up to the nearest Cargo.toml with a
# [package] table. A path outside any single package (e.g. the workspace
# root) means "analyze the whole workspace".

pkg_for_path() {
  local p="$1"
  local dir
  if [[ -d "$p" ]]; then dir="$p"; else dir="$(dirname "$p")"; fi
  dir="$(cd "$dir" && pwd)"
  while [[ "$dir" != "/" ]]; do
    if [[ -f "$dir/Cargo.toml" ]] && grep -q '^\[package\]' "$dir/Cargo.toml"; then
      grep -m1 '^name' "$dir/Cargo.toml" | sed -E 's/name *= *"(.*)"/\1/'
      return 0
    fi
    dir="$(dirname "$dir")"
  done
  return 1
}
