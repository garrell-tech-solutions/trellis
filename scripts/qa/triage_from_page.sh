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

# For capture_id's pool/committed/quota triage forms: whether the form sits
# inside a <details> (something that must be opened first), and how many
# fields a user must touch to submit it -- every non-hidden <input> and
# <select>, excluding the hidden kind input and the submit button, which
# qa/triage_from_page.md's "pool is the cheapest path" explicitly says not
# to count.
qa_extract_form_shapes() {
  local page="$1" capture_id="$2" block
  block="$(qa_capture_row_block "$page" "$capture_id")"
  python3 -c '
import re, json, sys
block = sys.argv[1]
details_blocks = re.findall(r"<details\b.*?</details>", block, re.S)

result = {}
for kind in ("pool", "committed", "quota"):
    form_m = re.search(r"<form\b[^>]*>(?:(?!</form>).)*?value=\"" + kind + r"\"(?:(?!</form>).)*?</form>", block, re.S)
    if not form_m:
        result[kind] = {"in_details": None, "field_count": None}
        continue
    form = form_m.group(0)
    inputs = [i for i in re.findall(r"<input\b[^>]*>", form) if "type=\"hidden\"" not in i]
    selects = re.findall(r"<select\b[^>]*>", form)
    result[kind] = {
        "in_details": any(form in details for details in details_blocks),
        "field_count": len(inputs) + len(selects),
    }

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
    qa_triage_form "$endpoint" "kind=pool&life_area=Work"
    if [[ "$STATUS" -ge 300 && "$STATUS" -lt 400 ]]; then
      echo "FAIL: [$name] pool triage redirected the browser (status $STATUS)" >&2
      FAILURES=1
    fi
    page="$(qa_get_inbox)"
    inbox="$(qa_html_section "$page" captures)"
    tasks="$(qa_html_section "$page" tasks)"
    if [[ "$inbox" == *"buy milk"* ]]; then
      echo "FAIL: [$name] \"buy milk\" still appears in the inbox after pool triage" >&2
      FAILURES=1
    fi
    if [[ "$tasks" != *"buy milk"* ]]; then
      echo "FAIL: [$name] \"buy milk\" does not appear in the task list after pool triage" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: committed rejection through the page ---
name="committed-rejected"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "call the dentist")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  endpoint="$(qa_json_field "$controls" committed_endpoint)"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the committed-triage control" >&2
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
# hidden_input_values checks.
name="committed-closed-choices"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "call the dentist")"
  block="$(qa_capture_row_block "$(qa_get_inbox)" "$capture_id")"
  committed_section="$(python3 -c '
import re, sys
block = sys.argv[1]
start_m = re.search(r"<summary>Committed</summary>", block)
if not start_m:
    print("")
    sys.exit()
pos = start_m.end()
depth = 1  # the outer <details> that already opened before this <summary>
i = pos
while i < len(block) and depth > 0:
    open_m = re.compile(r"<details\b").search(block, i)
    close_m = re.compile(r"</details>").search(block, i)
    if close_m and (not open_m or close_m.start() < open_m.start()):
        depth -= 1
        i = close_m.end()
        end = i
    elif open_m:
        depth += 1
        i = open_m.end()
    else:
        break
print(block[pos:end] if depth == 0 else "")
' "$block")"

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
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  endpoint="$(qa_json_field "$controls" quota_endpoint)"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the quota-triage control" >&2
    FAILURES=1
  else
    # target_count omitted; target_minutes_each and period supplied.
    qa_triage_form "$endpoint" "kind=quota&target_minutes_each=45&period=week"
    if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
      echo "FAIL: [$name] expected a client error, got status $STATUS" >&2
      FAILURES=1
    fi
    if [[ "$BODY" != *"target_count is required"* ]]; then
      echo "FAIL: [$name] expected the response to name target_count as required, got:
$BODY" >&2
      FAILURES=1
    fi
    task_count="$(qa_task_count)"
    if [[ "$task_count" != "0" ]]; then
      echo "FAIL: [$name] expected the task list to still be empty, found $task_count row(s)" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile capture text stays escaped in the task list ---
name="hostile-text-in-tasks"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "<script>alert('boom')</script>")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  endpoint="$(qa_json_field "$controls" pool_endpoint)"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the pool-triage control" >&2
    FAILURES=1
  else
    qa_triage_form "$endpoint" "kind=pool&life_area=Work"
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] setup triage returned status $STATUS, expected 201" >&2
      FAILURES=1
    fi
    tasks="$(qa_html_section "$(qa_get_inbox)" tasks)"
    if [[ "$tasks" == *"<script>"* ]]; then
      echo "FAIL: [$name] task list contains an unescaped <script> tag" >&2
      FAILURES=1
    fi
    if [[ "$tasks" != *"boom"* ]]; then
      echo "FAIL: [$name] task list does not contain the word \"boom\" -- content may have been stripped instead of escaped" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: pool is the cheapest path ---
name="pool-is-cheapest"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "buy milk")"
  shapes="$(qa_extract_form_shapes "$(qa_get_inbox)" "$capture_id")"
  pool_in_details="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["pool"]["in_details"])' "$shapes")"
  if [[ "$pool_in_details" != "False" ]]; then
    echo "FAIL: [$name] expected the pool control to be submittable straight from the row, not behind a control that must be opened first" >&2
    FAILURES=1
  fi
  pool_count="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["pool"]["field_count"])' "$shapes")"
  for kind in committed quota; do
    kind_count="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["'"$kind"'"]["field_count"])' "$shapes")"
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
