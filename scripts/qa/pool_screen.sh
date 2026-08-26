#!/usr/bin/env bash
# Executable QA procedure: qa/pool_screen.md (covers
# features/pool_screen.feature). Drives the running server through its
# HTTP interface only -- GET /pool and the capture/triage endpoints used to
# set it up -- and inspects nothing beyond what the page itself renders.
#
# T-qa-binds-tolerantly-to-markup governs this file, per the doc's own
# instruction: every extraction below binds to a class this template owns
# (trip panel, trip-tag, trip-count, trip-items, loose, loose-text,
# loose-tag, pool-meta), never to attribute order or adjacency, and never
# to quoted copy except where the doc itself quotes literal text.
#
# qa/pool_screen.md's by-hand walkthrough is NOT scripted here, for the
# usual reason (no browser automation) plus one this doc names itself: the
# distinction between reaching /pool "by its tab" and "by a typed URL" is
# invisible to curl, which can only ever GET the path directly. What is
# scripted is everything the doc's own numbered procedures ask for.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-pool-screen"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_get_pool() {
  curl -s "http://$ADDR/pool"
}

# Every trip-tag label, in document order.
qa_trip_tags_in_order() {
  local page="$1"
  python3 -c '
import re, sys
for m in re.finditer(r"<div class=\"trip-tag\">([^<]*)</div>", sys.argv[1]):
    print(m.group(1))
' "$page"
}

# --- Procedure: trips, strays, and the threshold ---
name="trips-strays-and-the-threshold"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@homedepot"
  qa_pool_task "return the drill" "@homedepot"
  qa_pool_task "pick up trim" "@homedepot"
  qa_pool_task "milk" "@supermarket"
  qa_pool_task "coffee" "@supermarket"
  qa_pool_task "fix the door latch"

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected one trip panel for @homedepot, found none" >&2
    FAILURES=1
  fi
  trip_count="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$trip_count" != "3 things" ]]; then
    echo "FAIL: [$name] expected the @homedepot trip to read \"3 things\", got: $trip_count" >&2
    FAILURES=1
  fi
  if [[ -n "$(qa_trip_section "$page" "@supermarket")" ]]; then
    echo "FAIL: [$name] @supermarket should not be a trip with only two items" >&2
    FAILURES=1
  fi

  loose="$(qa_loose_section "$page")"
  for text in "milk" "coffee"; do
    row="$(qa_loose_row_containing "$loose" "$text")"
    if [[ "$row" != *"@supermarket"* ]]; then
      echo "FAIL: [$name] expected the loose row for \"$text\" to still show @supermarket, got: $row" >&2
      FAILURES=1
    fi
  done
  latch_row="$(qa_loose_row_containing "$loose" "fix the door latch")"
  latch_text="$(qa_between "$latch_row" '<div class="loose-text">' '</div>')"
  if [[ "${latch_text//[[:space:]]/}" != "fixthedoorlatch" ]]; then
    echo "FAIL: [$name] expected \"fix the door latch\" to carry no tag, got row: $latch_row" >&2
    FAILURES=1
  fi

  meta="$(qa_between "$page" '<div class="pool-meta">' '</div>')"
  if [[ "$meta" != "6 waiting" ]]; then
    echo "FAIL: [$name] expected the meta count to read \"6 waiting\" (trips and loose together), got: $meta" >&2
    FAILURES=1
  fi

  # A third @supermarket item promotes the whole group to a trip.
  qa_pool_task "bread" "@supermarket"
  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@supermarket")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected @supermarket to become a trip once a third item arrived" >&2
    FAILURES=1
  fi
  loose="$(qa_loose_section "$page")"
  if [[ "$loose" == *"@supermarket"* ]]; then
    echo "FAIL: [$name] expected @supermarket's three items to have left loose ends, got: $loose" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: trip order ---
name="trip-order"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  # @cellar (4) created first, then @attic (3), then @bakery (3) -- creation
  # order, alphabetical order and size order all disagree with each other.
  for i in 1 2 3 4; do qa_pool_task "cellar item $i" "@cellar"; done
  for i in 1 2 3; do qa_pool_task "attic item $i" "@attic"; done
  for i in 1 2 3; do qa_pool_task "bakery item $i" "@bakery"; done

  order="$(qa_trip_tags_in_order "$(qa_get_pool)")"
  expected="@cellar
@attic
@bakery"
  if [[ "$order" != "$expected" ]]; then
    echo "FAIL: [$name] expected @cellar (most items) first, then @attic, @bakery alphabetically, got:
$order" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: one tag however it is spelled ---
name="one-tag-however-spelled"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@HomeDepot"
  qa_pool_task "return the drill" "@homedepot"
  qa_pool_task "pick up trim" "@HOMEDEPOT"

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@HomeDepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected one trip spelled @HomeDepot (first spelling used), got page:
$page" >&2
    FAILURES=1
  fi
  count="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count" != "3 things" ]]; then
    echo "FAIL: [$name] expected the three case-variant spellings to merge into one trip of three, got: $count" >&2
    FAILURES=1
  fi
  tag_mentions="$(printf '%s' "$page" | grep -o '@[Hh][Oo][Mm][Ee][Dd][Ee][Pp][Oo][Tt]' | grep -c .)"
  if [[ "$tag_mentions" != "1" ]]; then
    echo "FAIL: [$name] expected the tag mentioned exactly once (the trip heading), found $tag_mentions" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: only pool work appears ---
name="only-pool-work-appears"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@homedepot"
  committed_id="$(qa_submit_capture "return the drill")"
  qa_triage "$committed_id" '{"kind":"committed","deadline":"2026-08-28T17:00:00Z","commitment":"at","priority":"P1","estimated_minutes":60,"context_tag":"@homedepot"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup committed triage returned status $STATUS" >&2
    FAILURES=1
  fi
  quota_id="$(qa_submit_capture "pick up trim")"
  qa_triage "$quota_id" '{"kind":"quota","name":"pick up trim","hours":"2","context_tag":"@homedepot"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup quota triage returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_pool)"
  meta="$(qa_between "$page" '<div class="pool-meta">' '</div>')"
  if [[ "$meta" != "1 waiting" ]]; then
    echo "FAIL: [$name] expected only the pool task counted, got meta: $meta" >&2
    FAILURES=1
  fi
  for text in "return the drill" "pick up trim"; do
    if [[ "$page" == *"$text"* ]]; then
      echo "FAIL: [$name] \"$text\" (not a pool task) should not appear on the pool screen" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: order, and the absence of controls ---
name="order-and-absence-of-controls"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "first homedepot" "@homedepot"
  qa_pool_task "second homedepot" "@homedepot"
  qa_pool_task "third homedepot" "@homedepot"
  qa_pool_task "older untagged"
  qa_pool_task "newer untagged"

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  items="$(qa_between "$trip" '<ul class="trip-items">' '</ul>')"
  item_order="$(python3 -c 'import re,sys; print(",".join(re.findall(r"<span class=\"trip-item-text\">([^<]*)</span>", sys.argv[1])))' "$items")"
  if [[ "$item_order" != "third homedepot,second homedepot,first homedepot" ]]; then
    echo "FAIL: [$name] expected trip items newest first, got: $item_order" >&2
    FAILURES=1
  fi

  loose="$(qa_loose_section "$page")"
  loose_order="$(python3 -c 'import re,sys; print(",".join(re.findall(r"<div class=\"loose-text\">([^<]*)</div>", sys.argv[1])))' "$loose")"
  if [[ "$loose_order" != "newer untagged,older untagged" ]]; then
    echo "FAIL: [$name] expected loose ends newest first, got: $loose_order" >&2
    FAILURES=1
  fi

  # #97's own done checkbox legitimately carries hx-post now, so the
  # reorder check is scoped to the exact markers
  # pool_screen.rs's own then_no_reorder step checks -- not a blanket
  # hx-post/hx-put/hx-delete absence, which the checkbox would fail.
  for needle in "Raise priority" "Lower priority" "&#9650;" "&#9660;"; do
    if [[ "$page" == *"$needle"* ]]; then
      echo "FAIL: [$name] expected no reorder control anywhere on the page, found \"$needle\"" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a long trip ---
# #120 (pool-screen-truncation-06, revised): the old <details> nested the
# hidden items in a second list, and "3 shown" was answered by reading only
# the first <ul class="trip-items">. That two-list shape was the defect
# #120 removes -- one list holds every item now, and CSS alone decides how
# many paint. Over HTTP the trip holds all five; which three are actually
# visible on screen is a rendered-page fact and lives in
# scripts/qa/trip_controls.sh, not here.
name="a-long-trip"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for i in 1 2 3 4 5; do qa_pool_task "item $i" "@homedepot"; done

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  items="$(qa_between "$trip" '<ul class="trip-items">' '</ul>')"
  item_count="$(python3 -c 'import re,sys; print(len(re.findall(r"<li\b", sys.argv[1])))' "$items")"
  if [[ "$item_count" != "5" ]]; then
    echo "FAIL: [$name] expected the one list to hold all 5 items, got $item_count" >&2
    FAILURES=1
  fi
  if [[ "$trip" == *"<details"* || "$trip" == *"<summary"* ]]; then
    echo "FAIL: [$name] expected a plain button, not a <details>/<summary> disclosure, got: $trip" >&2
    FAILURES=1
  fi
  more_endpoint_marker='<button type="button" class="trip-more-toggle"'
  if [[ "$trip" != *"$more_endpoint_marker"* ]]; then
    echo "FAIL: [$name] expected a trip-more-toggle button, got: $trip" >&2
    FAILURES=1
  fi
  if [[ "$trip" != *"Show 2 more"* ]]; then
    echo "FAIL: [$name] expected the show-more control to read \"Show 2 more\", got: $trip" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "5 things" ]]; then
    echo "FAIL: [$name] expected the trip's count to read \"5 things\" throughout (what is waiting, not what is displayed), got: $count_label" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: nothing pooled ---
name="nothing-pooled"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_pool)"
  if [[ "$page" != *"Nothing in the pool"* ]]; then
    echo "FAIL: [$name] expected the empty-state message, got:
$page" >&2
    FAILURES=1
  fi
  if [[ "$page" != *'href="/"'* || "$page" != *"Go to Capture"* ]]; then
    echo "FAIL: [$name] expected a way back to Capture, got:
$page" >&2
    FAILURES=1
  fi
  meta="$(qa_between "$page" '<div class="pool-meta">' '</div>')"
  if [[ "$meta" != "empty" ]]; then
    echo "FAIL: [$name] expected the meta to read \"empty\" rather than a zero count, got: $meta" >&2
    FAILURES=1
  fi

  # A capture that exists but is untriaged does not count.
  qa_submit_capture "buy milk" >/dev/null
  page="$(qa_get_pool)"
  meta="$(qa_between "$page" '<div class="pool-meta">' '</div>')"
  if [[ "$meta" != "empty" ]]; then
    echo "FAIL: [$name] an untriaged capture should not count as pooled, got meta: $meta" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# The tab-bar procedure this file used to own (two tabs, Capture and Pool)
# was removed from features/pool_screen.feature under #94: it was only
# ever true while Pool was the last tab, and committed-screen-tabs-06 now
# asserts all three screens over all three tabs in one place. That check
# lives in scripts/qa/committed_screen.sh; restating a narrower version
# here is exactly the two-copies-drift qa/pool_screen.md's own "nothing
# else changed" section warns about (qa/pool_screen.md's prose itself is
# stale here and still describes the old two-tab scenario -- flagged to
# the specifier).

# --- Procedure: hostile text stays escaped ---
name="hostile-text-in-a-trip-heading"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  hostile="<script>alert('boom')</script>"
  qa_pool_task "buy screws" "$hostile"
  qa_pool_task "return the drill" "$hostile"
  qa_pool_task "pick up trim" "$hostile"

  page="$(qa_get_pool)"
  if [[ "$page" == *"<script>alert"* ]]; then
    echo "FAIL: [$name] the pool screen renders an unescaped <script> tag in a trip heading" >&2
    FAILURES=1
  fi
  if [[ "$page" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive in the trip heading, escaped rather than stripped" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

name="hostile-text-in-a-loose-tag"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  hostile="<script>alert('boom')</script>"
  qa_pool_task "buy screws" "$hostile"
  qa_pool_task "return the drill" "$hostile"

  page="$(qa_get_pool)"
  if [[ "$page" == *"<script>alert"* ]]; then
    echo "FAIL: [$name] the pool screen renders an unescaped <script> tag in a loose-end's tag label" >&2
    FAILURES=1
  fi
  if [[ "$page" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive in the loose-end tag label, escaped rather than stripped" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: pool_screen"
