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

# ---------------------------------------------------------------------------
# ONE instrumented test run, two reports.
#
# coverage.sh and crap.sh each need the same thing -- a profile of the whole
# suite under llvm instrumentation -- and each used to take its own. That is
# the expensive half of the `quality` job paid twice: measured on this tree,
# 142s for coverage.sh and 140s for crap.sh, and the second run learns nothing
# the first did not already know. They differ only in how they *render* the
# profile: coverage.sh exports `--json --summary-only`, crap.sh exports
# `--cobertura` for the per-method line rates it composes with complexity.
#
# `cargo llvm-cov --no-report` runs the tests and keeps the raw profile;
# `cargo llvm-cov report` renders that profile in any format and runs nothing.
# So the run happens once and each analyzer renders what it needs from it.
#
# TWO THINGS THIS MUST NOT BREAK, both Analyzer Contract requirements:
#
#   Standalone. Either analyzer must work with nothing else having run first.
#   On a miss the analyzer takes the run itself, exactly as it did before, so
#   the reuse is an optimisation and never a precondition.
#
#   Never stale. A reused profile that predates a source edit would report a
#   number about a tree that no longer exists -- plausible, well-formed and
#   wrong, which is the failure mode the ordering note in ci.yml is already
#   about. So the stamp records the scope the profile was taken under, and
#   anything the run reads being newer than the stamp discards it.
#
# The stamp lives under the cargo target directory, which is gitignored and
# which every analyzer's own tree walk excludes, so it is not itself something
# the analyzers can end up measuring.
ANALYZER_CACHE_DIR="${CARGO_TARGET_DIR:-target}/analyzers"
LLVM_COV_STAMP="$ANALYZER_CACHE_DIR/llvm-cov-profile"

# Inputs whose modification invalidates a profile: the sources that were
# compiled, the features the generated entry points are built from, the IR
# those entry points `include_str!`, and the manifests that decide what is in
# the workspace at all.
LLVM_COV_INPUTS=(crates features build/acceptance/ir Cargo.toml Cargo.lock)

llvm_cov_profile_fresh() {
  local key="$1" newer
  [[ -f "$LLVM_COV_STAMP" ]] || return 1
  [[ "$(cat "$LLVM_COV_STAMP" 2>/dev/null)" == "$key" ]] || return 1
  newer="$(find "${LLVM_COV_INPUTS[@]}" -newer "$LLVM_COV_STAMP" -print -quit \
    2>/dev/null || true)"
  [[ -z "$newer" ]]
}

# Usage: llvm_cov_profile_ensure <scope-key> <cargo llvm-cov scope args...>
#
# --include-ignored so property tests (run separately per stack.prompt, but
# tagged #[ignore] to stay out of the default `cargo test` run) still count
# toward coverage instead of reading as a false gap.
#
# --ignore-run-fail because this run measures; it does not adjudicate pass or
# fail. Coverage instrumentation makes the binary several times slower, and
# features/capture_endpoint.feature asserts a 50ms response budget -- measured
# on this machine at 746ms under instrumentation against 0.11s for the whole
# uninstrumented feature. That assertion is about the product's speed, and the
# instrumented build is not the product. Without this flag the analyzers fail
# intermittently with a timing error that says nothing about coverage: three
# consecutive runs on 2026-08-14 gave 101, 0, 0.
#
# It masks nothing. The generated entry points run every scenario to completion
# and collect the outcomes before asserting (see scripts/acceptance/generate.bb),
# so a failed assertion does not cut the walk short and the profile is complete
# either way. And the verdict on whether the tests pass is taken elsewhere, by
# `cargo test --workspace -- --include-ignored` in CI's `gate` job and by
# `cargo test -p acceptance-tests` in `quality` -- uninstrumented, at honest
# speed -- before this. A build failure still fails here, because that is not a
# run failure.
llvm_cov_profile_ensure() {
  local key="$1"
  shift
  if llvm_cov_profile_fresh "$key"; then
    return 0
  fi
  # Drop the stamp before the run, not after: a run that dies partway must not
  # leave a stamp claiming the profile beside it is complete.
  rm -f "$LLVM_COV_STAMP"
  cargo llvm-cov "$@" --no-report --ignore-run-fail >/dev/null 2>&1 \
    -- --include-ignored
  mkdir -p "$ANALYZER_CACHE_DIR"
  printf '%s\n' "$key" >"$LLVM_COV_STAMP"
}
