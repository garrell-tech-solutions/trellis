#!/usr/bin/env bash
# Executable QA procedure: qa/triage_from_page.md (covers
# features/triage_from_page.feature). Covers the automatable, curl-only
# procedures from qa/triage_from_page.md. Drives the running server through
# its HTTP interface only, and inspects persisted state via a read-only
# sqlite3 query -- never through a project-internal API.
#
# qa/triage_from_page.md's "By-hand walkthrough" is NOT scripted here, for
# the same reason inbox_view.sh doesn't script its own: this project's
# stack has no browser-automation tooling, and the document is explicit
# that the browser-visible half of "no full page reload" is a one-time
# human check curl cannot honestly replace. It has not been performed in a
# real browser this cycle; report it as unverified rather than implying it
# was checked.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-triage-from-page"
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

# Extracts capture_id's pool/committed/quota triage controls from a rendered
# page as one JSON object: each control's hx-post endpoint (read from the
# page's own markup, not assumed), plus the committed form's
# commitment/priority <select> options. Per qa/triage_from_page.md's
# "Independent of Implementation" note.
qa_extract_controls() {
  local page="$1" capture_id="$2" block
  block="$(qa_capture_row_block "$page" "$capture_id")"
  python3 -c '
import re, json, sys
block = sys.argv[1]

result = {}
for form in re.findall(r"<form\b[^>]*>.*?</form>", block, re.S):
    hx_post_m = re.search(r"hx-post=\"([^\"]+)\"", form)
    endpoint = hx_post_m.group(1) if hx_post_m else None
    if "value=\"pool\"" in form:
        result["pool_endpoint"] = endpoint
    elif "value=\"committed\"" in form:
        result["committed_endpoint"] = endpoint
        dt = re.search(r"<select name=\"commitment\">(.*?)</select>", form, re.S)
        result["commitment_options"] = re.findall(r"<option value=\"([^\"]+)\"", dt.group(1)) if dt else []
        pr = re.search(r"<select name=\"priority\">(.*?)</select>", form, re.S)
        result["priority_options"] = re.findall(r"<option value=\"([^\"]+)\"", pr.group(1)) if pr else []
    elif "value=\"quota\"" in form:
        result["quota_endpoint"] = endpoint

print(json.dumps(result))
' "$block"
}

# --- Procedure: the inbox offers all three kinds ---
name="offers-all-kinds"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "buy milk")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  for endpoint_field in pool_endpoint committed_endpoint quota_endpoint; do
    endpoint="$(qa_json_field "$controls" "$endpoint_field")"
    if [[ -z "$endpoint" ]]; then
      echo "FAIL: [$name] could not find a $endpoint_field control for the capture" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: pool triage through the page ---
name="pool-through-page"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "buy milk")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  endpoint="$(qa_json_field "$controls" pool_endpoint)"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the pool-triage control" >&2
    FAILURES=1
  else
    qa_triage_form "$endpoint" "kind=pool"
    if [[ "$STATUS" -ge 300 && "$STATUS" -lt 400 ]]; then
      echo "FAIL: [$name] pool triage redirected the browser (status $STATUS)" >&2
      FAILURES=1
    fi
    page="$(qa_get_inbox)"
    row="$(qa_capture_row_block "$page" "$capture_id")"
    if [[ "$row" != *"Pool · no context"* ]]; then
      echo "FAIL: [$name] expected \"buy milk\"'s Recent row restyled \"Pool · no context\" after pool triage, got: $row" >&2
      FAILURES=1
    fi
    if [[ "$row" == *"<button type=\"submit\">Pool</button>"* ]]; then
      echo "FAIL: [$name] expected no kind buttons on the triaged row, got: $row" >&2
      FAILURES=1
    fi
    pool_page="$(curl -s "http://$ADDR/pool")"
    if [[ "$pool_page" != *"buy milk"* ]]; then
      echo "FAIL: [$name] \"buy milk\" does not appear on the Pool screen after pool triage" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: committed rejection through the page ---
# #119 moved committed's fields behind a kind button: they no longer exist
# in the row's markup until POST .../kind has opened the panel (read from
# the row's kind-choice button, not assumed), so every procedure below that
# needs the committed or quota triage form now opens the panel first and
# re-reads the form's own endpoint from what comes back.
name="committed-rejected"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "call the dentist")"
  kind_endpoint="$(qa_row_control_endpoint "$(qa_get_inbox)" "$capture_id" 'value="committed"')"
  if [[ -z "$kind_endpoint" ]]; then
    echo "FAIL: [$name] could not find the committed kind-choice control" >&2
    FAILURES=1
  else
    qa_triage_form "$kind_endpoint" "kind=committed"
    endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$capture_id")" committed)"
    if [[ -z "$endpoint" ]]; then
      echo "FAIL: [$name] could not find the committed-triage control once the panel was open" >&2
      FAILURES=1
    else
      # deadline omitted; commitment and priority supplied.
      qa_triage_form "$endpoint" "kind=committed&commitment=at&priority=P1"
      if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
        echo "FAIL: [$name] expected a client error, got status $STATUS" >&2
        FAILURES=1
      fi
      if [[ "$BODY" != *"deadline is required"* ]]; then
        echo "FAIL: [$name] expected the response to name deadline as required, got:
$BODY" >&2
        FAILURES=1
      fi
      task_count="$(qa_task_count)"
      if [[ "$task_count" != "0" ]]; then
        echo "FAIL: [$name] expected the task list to still be empty, found $task_count row(s)" >&2
        FAILURES=1
      fi
      if qa_capture_untriaged "$capture_id"; then
        : # expected: still untriaged
      else
        echo "FAIL: [$name] expected the capture to still be untriaged" >&2
        FAILURES=1
      fi
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the committed form's closed choices ---
# #110 replaced commitment's <select> with two nested <details> disclosures
# ("At a time" / "By a day") -- the choice is which one the owner opens and
# submits, not a value picked from a dropdown
# (T-commitment-is-chosen-not-derived). Bound to the hidden commitment
# input each disclosure's own form carries, in document order -- the same
# thing crates/acceptance-tests/src/steps/triage_from_page.rs's
# hidden_input_values checks. #119 moved this pair out from under an outer
# "Committed" <details> into a plain <div class="fields-panel"> that only
# exists once the kind button has opened it -- both details are still
# unnamed siblings within it (commitment-choice exclusivity is untouched
# by #119 and remains a known, separately-tracked gap; see the handoff
# commit message).
name="committed-closed-choices"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "call the dentist")"
  kind_endpoint="$(qa_row_control_endpoint "$(qa_get_inbox)" "$capture_id" 'value="committed"')"
  qa_triage_form "$kind_endpoint" "kind=committed"
  block="$(qa_capture_row_block "$BODY" "$capture_id")"
  committed_section="$(qa_between "$block" '<div class="fields-panel">' '<form class="dismiss"')"

  commitment_choices="$(python3 -c '
import re, sys
section = sys.argv[1]
print(",".join(re.findall(r"<summary>([^<]*)</summary>", section)))
' "$committed_section")"
  if [[ "$commitment_choices" != "At a time,By a day" ]]; then
    echo "FAIL: [$name] expected the commitment choices \"At a time\" and \"By a day\" in that order, got \"$commitment_choices\"" >&2
    FAILURES=1
  fi

  commitment_values="$(python3 -c '
import re, sys
section = sys.argv[1]
print(",".join(re.findall(r"<input type=\"hidden\" name=\"commitment\" value=\"([^\"]*)\">", section)))
' "$committed_section")"
  if [[ "$commitment_values" != "at,by" ]]; then
    echo "FAIL: [$name] expected the two forms' own hidden commitment values to be exactly at,by, got \"$commitment_values\"" >&2
    FAILURES=1
  fi

  no_deadline_text_input="$(python3 -c '
import sys
print("yes" if "type=\"text\" name=\"deadline\"" in sys.argv[1] else "no")
' "$committed_section")"
  if [[ "$no_deadline_text_input" != "no" ]]; then
    echo "FAIL: [$name] expected no free-text deadline input anywhere in the committed section" >&2
    FAILURES=1
  fi

  priorities="$(python3 -c '
import re, sys
section = sys.argv[1]
selects = re.findall(r"<select name=\"priority\">(.*?)</select>", section, re.S)
options = [re.findall(r"<option value=\"([^\"]+)\"", s) for s in selects]
print(",".join(options[0]) if options else "")
' "$committed_section")"
  if [[ "$priorities" != "P1,P2,P3,P4" ]]; then
    echo "FAIL: [$name] expected the priority choices to be exactly P1,P2,P3,P4, got \"$priorities\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: quota rejection through the page ---
name="quota-rejected"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "go to the gym")"
  kind_endpoint="$(qa_row_control_endpoint "$(qa_get_inbox)" "$capture_id" 'value="quota"')"
  if [[ -z "$kind_endpoint" ]]; then
    echo "FAIL: [$name] could not find the quota kind-choice control" >&2
    FAILURES=1
  else
    qa_triage_form "$kind_endpoint" "kind=quota"
    endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$capture_id")" quota)"
    if [[ -z "$endpoint" ]]; then
      echo "FAIL: [$name] could not find the quota-triage control once the panel was open" >&2
      FAILURES=1
    else
      # hours omitted; name supplied.
      qa_triage_form "$endpoint" "kind=quota&name=workout"
      if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
        echo "FAIL: [$name] expected a client error, got status $STATUS" >&2
        FAILURES=1
      fi
      if [[ "$BODY" != *"hours is required"* ]]; then
        echo "FAIL: [$name] expected the response to name hours as required, got:
$BODY" >&2
        FAILURES=1
      fi
      task_count="$(qa_task_count)"
      if [[ "$task_count" != "0" ]]; then
        echo "FAIL: [$name] expected the task list to still be empty, found $task_count row(s)" >&2
        FAILURES=1
      fi
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile capture text stays escaped where it is rendered
# after triage --- #140: both halves (the pool screen and Recent's own
# restyled row), not just one -- a text-dropped-not-escaped bug could pass
# on one surface and not the other.
name="hostile-text-after-triage"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "<script>alert('boom')</script>")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  endpoint="$(qa_json_field "$controls" pool_endpoint)"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the pool-triage control" >&2
    FAILURES=1
  else
    qa_triage_form "$endpoint" "kind=pool"
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] setup triage returned status $STATUS, expected 201" >&2
      FAILURES=1
    fi
    pool_page="$(curl -s "http://$ADDR/pool")"
    if [[ "$pool_page" == *"<script>alert"* ]]; then
      echo "FAIL: [$name] the Pool screen contains an unescaped <script> tag" >&2
      FAILURES=1
    fi
    if [[ "$pool_page" != *"boom"* ]]; then
      echo "FAIL: [$name] the Pool screen does not contain the word \"boom\" -- content may have been stripped instead of escaped" >&2
      FAILURES=1
    fi
    recent_row="$(qa_capture_row_block "$(qa_get_inbox)" "$capture_id")"
    if [[ "$recent_row" == *"<script>alert"* ]]; then
      echo "FAIL: [$name] the restyled Recent row contains an unescaped <script> tag" >&2
      FAILURES=1
    fi
    if [[ "$recent_row" != *"boom"* ]]; then
      echo "FAIL: [$name] the restyled Recent row does not contain the word \"boom\" -- content may have been stripped instead of escaped" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# The non-hidden <input>/<select> count in section -- pool's own count
# comes straight from the row as viewed (D-pool-is-default: its form is
# present whether or not another kind's panel is open, and is never itself
# a <details>); committed/quota's count comes from the fields-panel once
# open, which is intentionally the whole open panel (committed's counts
# both its "at" and "by" forms together, matching the acceptance suite's
# own field_count/open_panel_section, which does not scope to a single
# nested form either).
qa_panel_field_count() {
  local section="$1"
  python3 -c '
import re, sys
section = sys.argv[1]
inputs = [i for i in re.findall(r"<input\b[^>]*>", section) if "type=\"hidden\"" not in i]
selects = re.findall(r"<select\b[^>]*>", section)
print(len(inputs) + len(selects))
' "$section"
}

# --- Procedure: pool is the cheapest path ---
name="pool-is-cheapest"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "buy milk")"
  page="$(qa_get_inbox)"
  block="$(qa_capture_row_block "$page" "$capture_id")"
  pool_section="$(qa_between "$block" '<form class="kind-choice kind-wide"' '</form>')"
  if [[ "$pool_section" == *"<details"* ]]; then
    echo "FAIL: [$name] expected the pool control to be submittable straight from the row, not behind a <details> that must be opened first" >&2
    FAILURES=1
  fi
  pool_count="$(qa_panel_field_count "$pool_section")"

  for kind in committed quota; do
    kind_endpoint="$(qa_row_control_endpoint "$page" "$capture_id" "value=\"$kind\"")"
    qa_triage_form "$kind_endpoint" "kind=$kind"
    panel_block="$(qa_capture_row_block "$BODY" "$capture_id")"
    panel_section="$(qa_between "$panel_block" '<div class="fields-panel">' '<form class="dismiss"')"
    kind_count="$(qa_panel_field_count "$panel_section")"
    if [[ -z "$pool_count" || -z "$kind_count" || "$pool_count" -ge "$kind_count" ]]; then
      echo "FAIL: [$name] expected pool ($pool_count inputs) to ask for strictly fewer inputs than $kind ($kind_count inputs)" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: triage_from_page"
