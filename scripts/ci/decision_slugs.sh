#!/usr/bin/env bash
# CI gate for docs/decisions.md: every decision slug cited in the source tree
# must exist as a row in the decisions log.
#
# Decisions used to be keyed by sequential number, and that failed twice in
# two days: two branches independently allocated T17, and then a third branch
# renumbered its source comments to a scheme that had never been settled --
# leaving every citation off by one. Because each wrong number still resolved
# to a *real* decision, nothing looked broken. A dangling reference announces
# itself; a reference to the wrong true thing does not.
#
# Slugs make the collision impossible. This gate makes the dangling reference
# loud: cite a decision that is not in the log and the build fails, naming the
# offending file and line.
# It cannot catch a citation of the wrong *existing* slug -- nothing can,
# short of reading the sentence -- but under slugs that mistake requires
# typing out someone else's decision by name rather than fat-fingering a
# digit.
#
# This is not a qa/*.md procedure -- it asserts a property of the repository,
# not of the running program -- so it lives under scripts/ci/ rather than
# scripts/qa/, and scripts/qa/run.sh does not pick it up. Same reasoning as
# migration_immutability.sh next door (T-migrations-append-only): a rule
# nobody checks is a comment.
#
# Usage: scripts/ci/decision_slugs.sh
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

LOG="docs/decisions.md"

# Where citations live. docs/ is deliberately absent: docs/decisions.md is the
# authority being checked against, and docs/plans/ holds dated handoff briefs
# that are historical records of what was true when they were written.
SEARCH_PATHS=(crates scripts qa features .github)

# What a citation looks like.
#
#   [TDR]      the three settled-decision prefixes. C/U/N are issue-tracker
#              prefixes, not rows in this file, so they are not ours to check.
#   -[a-z]     the tail must START with a lowercase letter, which rules out
#              `T-1`-style arithmetic and array slicing in scripts.
#   [a-z0-9-]  lowercase and digits only, so `T-complexity-8` matches while
#              `D-Day` does not.
#
# The requirement that the prefix be a capital letter and the tail be entirely
# lowercase is what keeps this off ordinary prose: mid-sentence `t-shirt`,
# `r-value` and `x-ray` do not match at all. The one near-miss in this repo's
# own vocabulary is the retired collective term for the technical decisions --
# capital T, hyphen, the word "series" -- which survives in one sentence of
# docs/decisions.md, a file this gate does not scan. It is spelled out here
# instead of written out because writing it would fail this gate, which is
# both the honest caveat and the demonstration.
#
# If such a word ever does turn up in a scanned file, reword it. There is
# deliberately no exception list, because an exception list is also where a
# genuinely broken citation would get hidden.
#
# Word boundaries do the rest: a slug at the end of a sentence, inside
# backticks, in a URL fragment (`decisions.md#T-rust`), or possessive
# (`T-module-boundary's`) all match exactly the slug and nothing else.
CITATION_RE='\b[TDR]-[a-z][a-z0-9]*(-[a-z0-9]+)*\b'

if [[ ! -f "$LOG" ]]; then
  echo "FAIL: $LOG not found -- nothing to validate citations against." >&2
  exit 1
fi

# The known slugs are the first cell of every table row in the log: the
# Settled and Rejected tables define them, and the "Former numbering" table
# lists them again against their retired numbers. Reading the rows rather
# than a hand-kept list means adding a decision needs no change here.
KNOWN="$(
  grep -E '^\|' "$LOG" \
    | sed -E 's/^\|[[:space:]]*//; s/[[:space:]]*\|.*$//; s/[`*]//g' \
    | grep -xE '[TDR]-[a-z][a-z0-9]*(-[a-z0-9]+)*' \
    | sort -u || true
)"

if [[ -z "$KNOWN" ]]; then
  echo "FAIL: no decision slugs found in $LOG." >&2
  echo "      Either the file's table format changed or it was truncated;" >&2
  echo "      an empty vocabulary would fail every citation in the tree." >&2
  exit 1
fi

# Tracked files plus new ones, minus anything gitignored -- so the generated
# acceptance entrypoints under crates/acceptance-tests/tests/ and build
# output are skipped, while a brand new feature file is not. -I skips
# binaries.
CITATIONS="$(
  git ls-files --cached --others --exclude-standard -z -- "${SEARCH_PATHS[@]}" \
    | xargs -0 --no-run-if-empty grep -IHnoE "$CITATION_RE" \
    | sort -u || true
)"
# `|| true` above is not laziness: grep exits 1 on a batch with no matches and
# xargs turns that into 123, so a tree that is merely clean would abort the
# script under `set -e`. A genuinely missing decisions.md is caught above, and
# an empty citation set is a pass, not an error.

UNKNOWN=""
while IFS= read -r hit; do
  [[ -z "$hit" ]] && continue
  slug="${hit##*:}"
  if ! grep -qxF "$slug" <<<"$KNOWN"; then
    UNKNOWN+="$hit"$'\n'
  fi
done <<<"$CITATIONS"

CITED_COUNT="$(grep -c . <<<"$CITATIONS" || true)"
KNOWN_COUNT="$(grep -c . <<<"$KNOWN" || true)"

if [[ -z "$UNKNOWN" ]]; then
  echo "PASS: decision_slugs ($CITED_COUNT citations in ${SEARCH_PATHS[*]}," \
    "all $KNOWN_COUNT slugs known to $LOG)"
  exit 0
fi

{
  echo "FAIL: these citations name a decision that is not in $LOG."
  echo
  while IFS= read -r hit; do
    [[ -z "$hit" ]] && continue
    file="${hit%%:*}"
    rest="${hit#*:}"
    line="${rest%%:*}"
    slug="${hit##*:}"
    echo "  $file:$line: unknown decision slug \"$slug\""
  done <<<"$UNKNOWN"
  cat <<EOF

WHY THIS IS BLOCKED

  A citation that resolves to nothing is a comment asserting a rule that no
  longer exists -- and the failure this gate was built for is worse than
  that: under the old sequential IDs a mistyped citation resolved to a real
  but *different* decision, so the comment read as correct while pointing
  somewhere else. Every reference here has to be one a reader can look up.

WHAT TO DO

  - Typo or stale number? Find the decision in $LOG and cite
    its slug. The "Former numbering" table there maps every retired T/D/R
    number to its slug -- but read the sentence you are fixing before
    trusting it, because the number in an old comment may itself be wrong.
  - Genuinely new decision? It is not settled until it has a row in
    $LOG. Add the row first (the PM owns that file), then
    cite it.
  - Not a citation at all? Reword it. A capitalised T-/D-/R- word with a
    lowercase tail reads as a decision reference to this gate and to anyone
    skimming the file.
EOF
} >&2
exit 1
