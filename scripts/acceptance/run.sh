#!/usr/bin/env bash
# Acceptance GENERATION (APS README "Pipeline"), up to but not including the
# run:
#   feature file -> gherkin parser -> JSON IR -> entrypoint generator
#   -> generated test entry points
#
# WHY THIS DOES NOT ALSO RUN THE SUITE
#
#   It used to end in `cargo test -p acceptance-tests`, and that made the
#   acceptance suite execute twice for every agent that verified its work.
#   `crates/acceptance-tests` is a workspace member, so once the entry points
#   exist on disk `cargo test --workspace` -- the command the constitution's
#   stack article names as "All" -- compiles and runs all of them. Running
#   them here as well meant the whole suite ran once here and again there:
#   the same assertions, the same ~30 seconds, twice, on every handoff.
#
#   Only one of the two could stop running them, and it could not be
#   `cargo test --workspace`: Cargo runs every integration test binary it
#   discovers and there is no way to ask it not to that does not also change
#   what the generated files are. So generation and running are separated
#   here, which is also what the constitution's engineering article already
#   describes ("Run acceptance generation and acceptance tests sequentially").
#
#   The local sequence is therefore:
#
#       ./scripts/acceptance/run.sh     # regenerate the entry points
#       cargo test --workspace          # unit + acceptance, once each
#
#   and CI's `quality` job spells out the same two steps. Nothing infers the
#   run from this script, so nothing can silently stop running it.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

IR_DIR="build/acceptance/ir"
DRY_DIR="build/acceptance/dry"
GEN_DIR="crates/acceptance-tests/tests"
META_DIR="$GEN_DIR/metadata"

mkdir -p "$IR_DIR" "$DRY_DIR" "$GEN_DIR" "$META_DIR"

# Generated acceptance tests are regenerated from features/*.feature every
# run; do not hand-edit anything under crates/acceptance-tests/tests/.
#
# WHY THIS NO LONGER DELETES THEM FIRST
#
#   The delete-then-regenerate shape was honest about the source of truth and
#   expensive in exactly the way nobody looks for. Generation is
#   deterministic: with no feature file changed, every regenerated entry point
#   and every regenerated IR file is byte-identical to the one it replaced.
#   Only the mtimes moved -- and mtimes are what Cargo rebuilds from, so a
#   no-op regeneration rebuilt all of the acceptance test binaries and
#   everything that `include_str!`s an IR file beside them. Measured on this
#   tree: `cargo test --workspace` 78s after a regeneration that changed
#   nothing, against 54s when the files were left alone.
#
#   So each output is now written only when its content actually differs (see
#   write-if-changed in generate.bb, and the IR comparison below), and stale
#   outputs are pruned at the end by name instead of by wiping the directory.
#   The invariant the wipe protected -- nothing survives here that no longer
#   has a feature behind it -- is unchanged, and the entry-point gate under
#   scripts/ci/ still checks it independently, in its own CI job.
EXPECTED_TESTS=()
EXPECTED_META=()

for feature in features/*.feature; do
  slug="$(basename "$feature" .feature)"
  ir_file="$IR_DIR/${slug}.json"
  dry_file="$DRY_DIR/${slug}.json"

  # The IR is `include_str!`d by the generated entry point, so its mtime is a
  # rebuild trigger just as much as the entry point's own. The parser writes
  # wherever it is told, so it is told to write beside the real file and the
  # result is moved into place only when it differs.
  ir_tmp="$(mktemp "$IR_DIR/.${slug}.json.XXXXXX")"
  trap 'rm -f "$ir_tmp"' EXIT
  ./scripts/acceptance/gherkin-parser.sh "$feature" "$ir_tmp"
  if [[ -f "$ir_file" ]] && cmp -s "$ir_tmp" "$ir_file"; then
    rm -f "$ir_tmp"
  else
    mv "$ir_tmp" "$ir_file"
  fi
  trap - EXIT

  ./scripts/acceptance/gherkin-ir-dry-checker.sh "$ir_file" "$dry_file"
  bb scripts/acceptance/generate.bb "$ir_file" "$GEN_DIR"

  EXPECTED_TESTS+=("${slug}_acceptance.rs")
  # generate.bb's metadata-name: lowercase "features/<slug>.feature" with every
  # run of non-alphanumerics collapsed to a single dash.
  meta_slug="$(printf '%s' "features/${slug}.feature" \
    | tr '[:upper:]' '[:lower:]' \
    | sed -E 's/[^a-z0-9]+/-/g; s/^-+|-+$//g')"
  EXPECTED_META+=("${meta_slug}.json")
done

# Prune whatever no longer has a feature behind it. A renamed or deleted
# feature leaves an entry point asserting something nobody wrote down, which
# is one of the two failure modes scripts/ci/acceptance_entrypoints.sh names.
prune_unexpected() {
  local dir="$1" pattern="$2"
  shift 2
  local expected=("$@") found base keep
  while IFS= read -r -d '' found; do
    base="$(basename "$found")"
    keep=0
    for e in "${expected[@]}"; do
      [[ "$base" == "$e" ]] && { keep=1; break; }
    done
    if [[ "$keep" -eq 0 ]]; then
      echo "pruned: $found"
      rm -f "$found"
    fi
  done < <(find "$dir" -maxdepth 1 -name "$pattern" -type f -print0)
}

prune_unexpected "$GEN_DIR" '*_acceptance.rs' "${EXPECTED_TESTS[@]}"
prune_unexpected "$META_DIR" '*.json' "${EXPECTED_META[@]}"

echo
echo "Entry points are up to date (${#EXPECTED_TESTS[@]} features)."
echo "Run them with:  cargo test --workspace"
