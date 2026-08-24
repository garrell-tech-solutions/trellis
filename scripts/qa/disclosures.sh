#!/usr/bin/env bash
# Executable QA procedure: qa/disclosures.md (covers
# features/disclosures.feature). Drives the running server through its
# HTTP interface only -- the inbox and the row-level kind/panel controls
# read from the page's own markup -- and inspects nothing beyond what the
# response actually renders.
#
# T-qa-binds-tolerantly-to-markup: every control is found by its own
# hx-post endpoint or a stable class this template owns (fields-panel,
# kind-choice), never by attribute order or adjacency; the old
# `.kind`/`.commitment-choice` <details> selectors this file's earlier
# incarnation (from #110's committed-date cycle) used for the KIND choice
# are gone and were never the contract -- #119 replaced that mechanism
# with three buttons and a server-remembered `shown_kind`.
#
# PRESENT VERSUS VISIBLE (qa/disclosures.md's own "which tool proves
# this?"): confirmed by reading crates/trellis-server/templates/
# capture_row.html and trellis.css directly before writing this script --
# each kind's fields-panel is wrapped in an Askama `{% if %}`, so the
# other kind's fields are textually ABSENT from the markup, not merely
# CSS-hidden (no `display: none` or `visibility: hidden` touches
# `.fields-panel` anywhere in trellis.css). HTTP is therefore sufficient
# for every exclusivity assertion below; no browser check is needed for
# it, unlike qa/committed_date.md's `phone_layout.cjs` extension.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-disclosures"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_get_inbox() {
  curl -s "http://$ADDR/"
}

# Markers distinct enough to appear only inside one kind's own panel
# content -- mirroring crates/acceptance-tests/src/steps/disclosures.rs's
# own kind_panel_marker, the same reference the acceptance suite uses.
qa_kind_marker() {
  case "$1" in
    committed) echo "At a time" ;;
    quota) echo "Sessions" ;;
    *) echo "" ;;
  esac
}

# Chooses kind for capture_id: POSTs to the row's own kind-choice button
# endpoint (read from page, not assumed) and sets STATUS/BODY to the
# re-rendered #lists fragment.
qa_choose_kind() {
  local page="$1" capture_id="$2" kind="$3" endpoint
  endpoint="$(qa_row_control_endpoint "$page" "$capture_id" "value=\"$kind\"")"
  if [[ -z "$endpoint" ]]; then
    echo "no kind-choice control found for capture $capture_id, kind $kind" >&2
    return 1
  fi
  qa_triage_form "$endpoint" "kind=$kind"
}

# --- Procedure: one panel, and only within the row ---
name="one-panel-and-only-within-the-row"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  first_id="$(qa_submit_capture "buy screws")"
  second_id="$(qa_submit_capture "call the dentist")"

  qa_choose_kind "$(qa_get_inbox)" "$first_id" committed
  first_row="$(qa_capture_row_block "$BODY" "$first_id")"
  if [[ "$first_row" != *"At a time"* ]]; then
    echo "FAIL: [$name] after choosing Committed on the first row, expected its committed fields, got:
$first_row" >&2
    FAILURES=1
  fi
  if [[ "$first_row" == *"Sessions"* ]]; then
    echo "FAIL: [$name] after choosing Committed on the first row, its quota fields are also present:
$first_row" >&2
    FAILURES=1
  fi

  qa_choose_kind "$BODY" "$first_id" quota
  first_row="$(qa_capture_row_block "$BODY" "$first_id")"
  if [[ "$first_row" == *"At a time"* ]]; then
    echo "FAIL: [$name] after choosing Quota on the first row, its committed fields are still present (stacked, not replaced):
$first_row" >&2
    FAILURES=1
  fi
  if [[ "$first_row" != *"Sessions"* ]]; then
    echo "FAIL: [$name] after choosing Quota on the first row, expected its quota fields, got:
$first_row" >&2
    FAILURES=1
  fi

  qa_choose_kind "$BODY" "$second_id" committed
  second_row="$(qa_capture_row_block "$BODY" "$second_id")"
  first_row="$(qa_capture_row_block "$BODY" "$first_id")"
  if [[ "$second_row" != *"At a time"* ]]; then
    echo "FAIL: [$name] after choosing Committed on the second row, expected its committed fields, got:
$second_row" >&2
    FAILURES=1
  fi
  if [[ "$first_row" != *"Sessions"* ]]; then
    echo "FAIL: [$name] THE TRAP: choosing Committed on the second row disturbed the first row -- expected it to still show Quota's fields, got:
$first_row" >&2
    FAILURES=1
  fi
  if [[ "$first_row" == *"At a time"* ]]; then
    echo "FAIL: [$name] THE TRAP: choosing Committed on the second row leaked committed fields into the first row:
$first_row" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# A page-form rejection's body is the re-rendered row fragment, not JSON
# (qa_assert_rejected_naming assumes the JSON API's {"missing_field": ...}
# shape and does not apply here) -- asserts client error, the field named
# in the row's own error text ("X is required", triage/http.rs's own page
# rendering), and durable state unchanged.
qa_assert_page_rejected_naming() {
  local name="$1" field_name="$2"
  if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
    echo "FAIL: [$name] expected a client error, got status $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"$field_name is required"* ]]; then
    echo "FAIL: [$name] expected the rejection to name $field_name, got:
$BODY" >&2
    FAILURES=1
  fi
  qa_assert_durable_state_unchanged "$name"
}

# --- Procedure: nothing about submission changed ---
name="nothing-about-submission-changed"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  # Committed, each required field omitted in turn.
  for missing in deadline commitment priority estimated_minutes; do
    CAPTURE_ID="$(qa_submit_capture "call the dentist $missing")"
    qa_choose_kind "$(qa_get_inbox)" "$CAPTURE_ID" committed
    endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$CAPTURE_ID")" committed)"
    declare -A fields=([deadline_date]="2026-09-01" [commitment]="by" [priority]="P1" [estimated_minutes]="30")
    field_name="$missing"
    [[ "$missing" == "deadline" ]] && field_name="deadline_date"
    unset "fields[$field_name]"
    data="kind=committed"
    for k in "${!fields[@]}"; do
      data="$data&$k=${fields[$k]}"
    done
    qa_triage_form "$endpoint" "$data"
    qa_assert_page_rejected_naming "$name-committed-$missing" "$missing"
  done

  # Quota, target_count omitted.
  CAPTURE_ID="$(qa_submit_capture "go to the gym")"
  qa_choose_kind "$(qa_get_inbox)" "$CAPTURE_ID" quota
  endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$CAPTURE_ID")" quota)"
  qa_triage_form "$endpoint" "kind=quota&target_minutes_each=45&period=week"
  qa_assert_page_rejected_naming "$name-quota-target_count" target_count

  # A complete committed and a complete quota triage still succeed, and
  # each carries only its own kind's fields -- two forms never merged
  # (qa/disclosures.md's "if the forms have been consolidated ... a
  # cosmetic defect has become a data defect").
  committed_id="$(qa_submit_capture "file taxes")"
  qa_choose_kind "$(qa_get_inbox)" "$committed_id" committed
  endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$committed_id")" committed)"
  panel="$(qa_between "$(qa_capture_row_block "$BODY" "$committed_id")" '<div class="fields-panel">' '<form class="dismiss"')"
  if [[ "$panel" == *"target_count"* || "$panel" == *"target_minutes_each"* ]]; then
    echo "FAIL: [$name] the committed panel carries a quota field -- the forms have been merged:
$panel" >&2
    FAILURES=1
  fi
  qa_triage_form "$endpoint" "kind=committed&commitment=by&deadline_date=2026-09-01&priority=P1&estimated_minutes=30"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a complete committed submission returned status $STATUS" >&2
    FAILURES=1
  fi

  quota_id="$(qa_submit_capture "read a book")"
  qa_choose_kind "$(qa_get_inbox)" "$quota_id" quota
  quota_panel="$(qa_between "$(qa_capture_row_block "$BODY" "$quota_id")" '<div class="fields-panel">' '<form class="dismiss"')"
  if [[ "$quota_panel" == *"deadline"* || "$quota_panel" == *"commitment"* ]]; then
    echo "FAIL: [$name] the quota panel carries a committed field -- the forms have been merged:
$quota_panel" >&2
    FAILURES=1
  fi
  endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$quota_id")" quota)"
  qa_triage_form "$endpoint" "kind=quota&target_count=3&target_minutes_each=45&period=week"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a complete quota submission returned status $STATUS" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: disclosures"
