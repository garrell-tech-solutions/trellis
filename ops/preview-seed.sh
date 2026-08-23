#!/usr/bin/env bash
# ops/preview-seed.sh <base-url>
#
# Fills a freshly-migrated, empty preview database with exactly the states
# issue #102 named as unverifiable by any gate that reads a computed style or
# a rendered screen for a living: a Pool trip, Pool loose ends below the trip
# threshold, an untriaged Capture inbox, a Committed "at", a Committed "by",
# and one overdue Committed item that must still show, marked, and first
# (T-trips-are-derived-not-ranked; committed_screen's own doc comment: "the
# one failure this screen cannot have is a missed deadline that silently
# vanishes").
#
# Driven entirely through the app's own HTTP surface -- POST /captures, then
# POST /captures/{id}/triage, the same two endpoints the quick-add box and
# the page's triage controls use -- rather than writing rows into preview.db
# directly. This is the deliberate answer to issue #102's second open
# question. A hand-written .sql fixture is easier and drifts from the schema
# silently; T-cross-capability-invariants-need-an-owner is exactly the
# failure mode that produces: a pool fixture that wrote a tag with the
# spelling already final skipped capture::resolve_tag's canonicalisation, two
# spellings of one tag split a would-be trip into groups of 2 and 1, and a
# trip vanished from the Pool screen with no test under it. Every row this
# script creates goes through the same validation, canonicalisation and
# rejection contract a real triage would -- it cannot express a state the
# app itself would refuse.
#
# The cost is that this script is exactly as sensitive to the branch's own
# HTTP contract as a real user would be. If a branch's migrations change what
# triage requires (a new required field, a renamed one) this script's fixed
# payload can stop satisfying it. `curl -sf` and `set -e` turn that into a
# loud, immediate failure -- the reseed step of ops/preview.sh aborts rather
# than leaving a half-seeded preview or silently seeding a state the branch
# no longer means. That failure is itself informative: it says the branch
# changed a contract this fixture assumed, which is a fact worth surfacing,
# not papering over.
set -euo pipefail

BASE="${1:?usage: preview-seed.sh <base-url>}"

# Extracts the numeric id out of `id="capture-row-<id>"` in a just-created
# capture's own response markup
# (crates/trellis-server/templates/capture_row.html). A JSON POST /captures
# deliberately never returns the new id (see
# a_json_submission_still_gets_back_a_bare_201_with_no_body in
# crates/trellis-server/src/capture/http.rs), so this seeds through the form
# transport instead -- the same one the quick-add box uses, and the only one
# that hands back the id the next request needs.
capture() {
  local raw_text="$1" tag="${2:-}" body
  if [[ -n "$tag" ]]; then
    body="$(curl -sf -X POST "$BASE/captures" \
      --data-urlencode "raw_text=$raw_text" \
      --data-urlencode "source=preview-seed" \
      --data-urlencode "context_tag=$tag")"
  else
    body="$(curl -sf -X POST "$BASE/captures" \
      --data-urlencode "raw_text=$raw_text" \
      --data-urlencode "source=preview-seed")"
  fi
  grep -oE 'capture-row-[0-9]+' <<<"$body" | head -1 | grep -oE '[0-9]+'
}

triage() {
  local id="$1"
  shift
  curl -sf -X POST "$BASE/captures/$id/triage" "$@" >/dev/null
}

pool_item() {
  local text="$1" tag="${2:-}" id
  id="$(capture "$text" "$tag")"
  triage "$id" --data-urlencode "kind=pool"
}

committed_item() {
  local text="$1" deadline="$2" commitment="$3" priority="$4" minutes="$5" id
  id="$(capture "$text" "")"
  triage "$id" \
    --data-urlencode "kind=committed" \
    --data-urlencode "deadline=$deadline" \
    --data-urlencode "commitment=$commitment" \
    --data-urlencode "priority=$priority" \
    --data-urlencode "estimated_minutes=$minutes"
}

# --- Pool: a trip at @homedepot (>= scheduler_core::pool::TRIP_THRESHOLD,
# 3), loose ends below it at @supermarket, and one untagged loose item -----
pool_item "Buy 2x4s"             "@homedepot"
pool_item "Return the drill"     "@homedepot"
pool_item "Pick up wood screws"  "@homedepot"
pool_item "Milk"                 "@supermarket"
pool_item "Eggs"                 "@supermarket"
pool_item "Fix the door latch"   ""

# --- Committed: an "at" with a time, a "by", and one overdue --------------
NOW_EPOCH="$(date -u +%s)"
AT_DEADLINE="$(date -u -d "@$((NOW_EPOCH + 2 * 86400))" +%Y-%m-%dT14:00:00Z)"
# committed_screen::date_cell renders a "by" as "BY <weekday>" only --
# T-commitment-is-chosen-not-derived and committed-screen-at-and-by-02 keep a
# by's time of day out of the cell on purpose, so it never leaks in and get
# mistaken for an "at". The 17:00 deadline is still seeded underneath it, so
# what does not render is a deliberate finding this branch confirmed, not a
# gap in the seed.
BY_DEADLINE="$(date -u -d "next thursday 17:00" +%Y-%m-%dT%H:%M:%SZ)"
OVERDUE_DEADLINE="$(date -u -d "@$((NOW_EPOCH - 86400))" +%Y-%m-%dT12:00:00Z)"

committed_item "Q3 planning doc"          "$AT_DEADLINE"      "at" "P2" 60
committed_item "File the tax return"      "$BY_DEADLINE"      "by" "P1" 30
committed_item "Renew the parking permit" "$OVERDUE_DEADLINE" "at" "P1" 15

# --- Capture: a couple of untriaged captures, so the inbox is not empty ---
capture "Look into the roof quote" "" >/dev/null
capture "Ask Sam about the offsite" "" >/dev/null

echo "seeded $BASE"
