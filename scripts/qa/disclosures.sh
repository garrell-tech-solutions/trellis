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
    # #138 retired the quota panel's target_count/target_minutes_each/period
    # fields (and with them the last text this panel shared with no other
    # kind's markup); "Hours a week" is its own label now.
    quota) echo "Hours a week" ;;
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
  if [[ "$first_row" == *"Hours a week"* ]]; then
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
  if [[ "$first_row" != *"Hours a week"* ]]; then
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
  if [[ "$first_row" != *"Hours a week"* ]]; then
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

  # Quota, hours omitted.
  CAPTURE_ID="$(qa_submit_capture "go to the gym")"
  qa_choose_kind "$(qa_get_inbox)" "$CAPTURE_ID" quota
  endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$CAPTURE_ID")" quota)"
  qa_triage_form "$endpoint" "kind=quota&name=go+to+the+gym"
  qa_assert_page_rejected_naming "$name-quota-hours" hours

  # A complete committed and a complete quota triage still succeed, and
  # each carries only its own kind's fields -- two forms never merged
  # (qa/disclosures.md's "if the forms have been consolidated ... a
  # cosmetic defect has become a data defect").
  committed_id="$(qa_submit_capture "file taxes")"
  qa_choose_kind "$(qa_get_inbox)" "$committed_id" committed
  endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$committed_id")" committed)"
  panel="$(qa_between "$(qa_capture_row_block "$BODY" "$committed_id")" '<div class="fields-panel">' '<form class="dismiss"')"
  if [[ "$panel" == *'name="hours"'* ]]; then
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
  qa_triage_form "$endpoint" "kind=quota&name=Reading&hours=3"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a complete quota submission returned status $STATUS" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# The quota panel's <input name="name"> value, or "" if no quota panel is
# open in block.
qa_quota_name_value() {
  local block="$1"
  python3 -c '
import re, sys
m = re.search(r"<input type=\"text\" name=\"name\" value=\"([^\"]*)\">", sys.argv[1])
print(m.group(1) if m else "")
' "$block"
}

# --- Procedure: the quota panel's name box (#138) ---
# HTTP-only, not the browser: "editable" is checkable as the absence of a
# readonly/disabled attribute, and the unsubmitted-edit-does-not-persist
# claim needs nothing typed client-side to prove -- the server never
# receives a candidate name until the form is actually submitted, so
# switching kind away and back can only ever re-render from capture.text
# unless a draft column exists to check for instead
# (qa/quota_triage_validation.md's browser script covers the actual
# keystroke-level as-you-type and reload behaviour).
name="the-quota-panels-name-box"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  lev_id="$(qa_submit_capture "learning with lev")"
  piano_id="$(qa_submit_capture "practise piano")"

  qa_choose_kind "$(qa_get_inbox)" "$lev_id" quota
  lev_block="$(qa_capture_row_block "$BODY" "$lev_id")"
  lev_panel="$(qa_between "$lev_block" '<div class="fields-panel">' '<form class="dismiss"')"
  name_value="$(qa_quota_name_value "$lev_panel")"
  if [[ "$name_value" != "learning with lev" ]]; then
    echo "FAIL: [$name] expected the name box prefilled with the full capture text \"learning with lev\", got: \"$name_value\"" >&2
    FAILURES=1
  fi
  if [[ "$lev_panel" == *"readonly"* || "$lev_panel" == *"disabled"* ]]; then
    echo "FAIL: [$name] expected the name box to be editable (no readonly/disabled), got:
$lev_panel" >&2
    FAILURES=1
  fi

  # Step 2: choosing Quota on the second row must not disturb the first.
  qa_choose_kind "$BODY" "$piano_id" quota
  lev_block="$(qa_capture_row_block "$BODY" "$lev_id")"
  lev_panel="$(qa_between "$lev_block" '<div class="fields-panel">' '<form class="dismiss"')"
  if [[ -z "$lev_panel" ]]; then
    echo "FAIL: [$name] expected the first row's quota panel still open after choosing Quota on the second row" >&2
    FAILURES=1
  fi
  name_value="$(qa_quota_name_value "$lev_panel")"
  if [[ "$name_value" != "learning with lev" ]]; then
    echo "FAIL: [$name] expected the first row's name box unmoved by the second row's action, got: \"$name_value\"" >&2
    FAILURES=1
  fi
  piano_block="$(qa_capture_row_block "$BODY" "$piano_id")"
  piano_panel="$(qa_between "$piano_block" '<div class="fields-panel">' '<form class="dismiss"')"
  name_value="$(qa_quota_name_value "$piano_panel")"
  if [[ "$name_value" != "practise piano" ]]; then
    echo "FAIL: [$name] expected the second row's own prefill \"practise piano\", got: \"$name_value\"" >&2
    FAILURES=1
  fi

  # Step 4: switching the first row away to Committed and back to Quota
  # must not leave anything behind that was never submitted -- confirmed
  # by checking the schema for a draft-name column, and confirming the
  # panel re-renders the plain prefill either way.
  draft_columns="$(sqlite3 "$DB_PATH" "PRAGMA table_info(captures);" | grep -ci "draft\|pending_name" || true)"
  if [[ "$draft_columns" != "0" ]]; then
    echo "FAIL: [$name] found a column on captures that looks like it holds a draft/pending name -- T-migrations-append-only means this can never be taken back, report it before anything else" >&2
    FAILURES=1
  fi
  qa_choose_kind "$BODY" "$lev_id" committed
  qa_choose_kind "$BODY" "$lev_id" quota
  lev_block="$(qa_capture_row_block "$BODY" "$lev_id")"
  lev_panel="$(qa_between "$lev_block" '<div class="fields-panel">' '<form class="dismiss"')"
  name_value="$(qa_quota_name_value "$lev_panel")"
  if [[ "$name_value" != "learning with lev" ]]; then
    echo "FAIL: [$name] expected the name box back to its own prefill \"learning with lev\" after switching kind away and back, got: \"$name_value\"" >&2
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
