#!/usr/bin/env bash
# Normal acceptance run (APS README "Pipeline"):
#   feature file -> gherkin parser -> JSON IR -> entrypoint generator
#   -> generated test entry points -> project test runner
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

IR_DIR="build/acceptance/ir"
DRY_DIR="build/acceptance/dry"
GEN_DIR="crates/acceptance-tests/tests"

mkdir -p "$IR_DIR" "$DRY_DIR" "$GEN_DIR"

# Generated acceptance tests are regenerated from features/*.feature every
# run; do not hand-edit anything under crates/acceptance-tests/tests/.
find "$GEN_DIR" -maxdepth 1 -name '*_acceptance.rs' -delete
rm -rf "$GEN_DIR/metadata"

for feature in features/*.feature; do
  slug="$(basename "$feature" .feature)"
  ir_file="$IR_DIR/${slug}.json"
  dry_file="$DRY_DIR/${slug}.json"

  ./scripts/acceptance/gherkin-parser.sh "$feature" "$ir_file"
  ./scripts/acceptance/gherkin-ir-dry-checker.sh "$ir_file" "$dry_file"
  bb scripts/acceptance/generate.bb "$ir_file" "$GEN_DIR"
done

cargo test -p acceptance-tests
