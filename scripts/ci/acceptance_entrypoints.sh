#!/usr/bin/env bash
# CI gate for issue #26: every features/*.feature must have produced a
# generated acceptance entrypoint, and no entrypoint may exist without a
# feature behind it.
#
# WHY THIS EXISTS SEPARATELY FROM RUNNING THE SUITE
#
#   The suite's failure mode is silence. `.gitignore` excludes
#   crates/acceptance-tests/tests/*_acceptance.rs, correctly -- the .feature
#   file is the source of truth and scripts/acceptance/run.sh regenerates the
#   entrypoints from it. But that means a fresh clone has no tests/ directory
#   at all, Cargo discovers no integration test binaries, and
#   `cargo test --workspace` prints `ok` having run unit tests only. It does
#   not warn, and it does not compare its test count against an expected one.
#
#   run.sh ends in `cargo test -p acceptance-tests`, so if generation produces
#   nothing, that command finds nothing, passes, and run.sh exits 0. The
#   acceptance step goes green while asserting nothing whatsoever. This gate is
#   the check that makes that state loud: it counts, by name, what generation
#   was supposed to produce against what it did.
#
#   It is deliberately cheap and deliberately independent of the APS toolchain.
#   It needs no babashka, no APS clone and no Rust toolchain, so it keeps
#   working as a guard even if the generation step ahead of it is broken or
#   uninstallable.
#
# This is not a qa/*.md procedure -- it asserts a property of the repository,
# not of the running program -- so it lives under scripts/ci/ rather than
# scripts/qa/, and scripts/qa/run.sh does not pick it up. Same reasoning as
# migration_immutability.sh and decision_slugs.sh next door.
#
# Usage: scripts/ci/acceptance_entrypoints.sh
#        Run it AFTER scripts/acceptance/run.sh, never before.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

FEATURE_DIR="features"
GEN_DIR="crates/acceptance-tests/tests"
META_DIR="$GEN_DIR/metadata"

# Slugs, by convention: features/<slug>.feature produces
# <slug>_acceptance.rs. scripts/acceptance/generate.bb reconstructs the feature
# path from the IR filename the same way, so this is the pipeline's own naming
# rule rather than a second one invented here.
FEATURES="$(
  find "$FEATURE_DIR" -maxdepth 1 -name '*.feature' -type f -printf '%f\n' 2>/dev/null \
    | sed -E 's/\.feature$//' | sort || true
)"
GENERATED="$(
  find "$GEN_DIR" -maxdepth 1 -name '*_acceptance.rs' -type f -printf '%f\n' 2>/dev/null \
    | sed -E 's/_acceptance\.rs$//' | sort || true
)"

FEATURE_COUNT="$(grep -c . <<<"$FEATURES" || true)"
GENERATED_COUNT="$(grep -c . <<<"$GENERATED" || true)"

# Floor. Zero features would make every comparison below trivially satisfied,
# so an empty features/ directory has to fail rather than pass quietly -- the
# same vacuous-pass shape this gate was built to catch one level down.
if [[ "$FEATURE_COUNT" -eq 0 ]]; then
  {
    echo "FAIL: no $FEATURE_DIR/*.feature files found."
    echo "      With no features there is nothing to generate and nothing to"
    echo "      compare, so a pass here would assert nothing. Either the"
    echo "      directory moved or the checkout is incomplete."
  } >&2
  exit 1
fi

MISSING="$(comm -23 <(echo "$FEATURES") <(echo "$GENERATED") || true)"
ORPHANED="$(comm -13 <(echo "$FEATURES") <(echo "$GENERATED") || true)"

# Every generated entrypoint should also have a metadata file recording its
# implementation hash (generate.bb writes both). A missing metadata directory
# means the generator was interrupted partway, which the slug comparison alone
# would not notice.
META_COUNT="$(find "$META_DIR" -maxdepth 1 -name '*.json' -type f 2>/dev/null | wc -l)"

if [[ -z "$MISSING" && -z "$ORPHANED" && "$META_COUNT" -eq "$GENERATED_COUNT" ]]; then
  echo "PASS: acceptance_entrypoints ($FEATURE_COUNT features," \
    "$GENERATED_COUNT generated entrypoints, $META_COUNT metadata files," \
    "slugs match one for one)"
  exit 0
fi

{
  echo "FAIL: the generated acceptance entrypoints do not match $FEATURE_DIR/*.feature."
  echo
  printf '  %-52s %d\n' "$FEATURE_DIR/*.feature" "$FEATURE_COUNT"
  printf '  %-52s %d\n' "$GEN_DIR/*_acceptance.rs" "$GENERATED_COUNT"
  printf '  %-52s %d\n' "$META_DIR/*.json" "$META_COUNT"
  echo

  if [[ -n "$MISSING" ]]; then
    echo "  Features with no generated entrypoint:"
    while IFS= read -r slug; do
      [[ -z "$slug" ]] && continue
      echo "    $FEATURE_DIR/$slug.feature -> $GEN_DIR/${slug}_acceptance.rs is absent"
    done <<<"$MISSING"
    echo
  fi

  if [[ -n "$ORPHANED" ]]; then
    echo "  Generated entrypoints with no feature behind them:"
    while IFS= read -r slug; do
      [[ -z "$slug" ]] && continue
      echo "    $GEN_DIR/${slug}_acceptance.rs -> $FEATURE_DIR/$slug.feature is absent"
    done <<<"$ORPHANED"
    echo
  fi

  if [[ "$META_COUNT" -ne "$GENERATED_COUNT" ]]; then
    echo "  $META_COUNT metadata files for $GENERATED_COUNT entrypoints -- generation"
    echo "  did not finish, or something wrote an entrypoint without going"
    echo "  through scripts/acceptance/generate.bb."
    echo
  fi

  cat <<EOF
WHY THIS IS BLOCKED

  The Gherkin suite is how acceptance criteria are enforced in this project,
  and its failure mode is silence rather than noise. The generated entrypoints
  are gitignored on purpose, so nothing in a clone tells you they are missing:
  Cargo simply discovers no integration test binaries, \`cargo test\` prints
  \`ok\`, and the green check on the pull request means strictly less than a
  reader believes it does.

  A feature with no entrypoint is a specification that no longer runs. An
  entrypoint with no feature is a test asserting something nobody wrote down.
  Neither announces itself.

WHAT TO DO

  - Ran the suite? Run it again and read the output:

        ./scripts/acceptance/run.sh

    It regenerates every entrypoint from features/*.feature. If it exits 0 and
    this gate still fails, generation itself is broken -- which is the case
    this gate exists for.
  - Added a feature? It is only wired up once run.sh has generated its
    entrypoint. Do not hand-write one; nothing under $GEN_DIR is
    hand-maintained.
  - Deleted or renamed a feature? Stale entrypoints are removed by run.sh,
    which deletes $GEN_DIR/*_acceptance.rs before regenerating. Run it.
EOF
} >&2
exit 1
