#!/usr/bin/env bash
# Executable QA procedure: qa/context_tags.md (covers
# features/context_tags.feature). Drives the running server through its
# HTTP interface only -- the inbox at GET /, its quick-add box (endpoint
# read from the page's own markup, not assumed), and the triage controls on
# each capture row -- and inspects persisted state via a read-only sqlite3
# query, never a project-internal API.
#
# qa/context_tags.md's by-hand browser walkthrough is NOT scripted here, for
# the reason its own doc names: there is no browser-automation tooling in
# this environment, and prefix narrowing on the datalist is specifically
# called out as unverifiable outside a real browser. What is scripted below
# is what the doc says its automated half can prove: the server sends every
# prior tag, exactly once, in a datalist the browser then narrows.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-context-tags"
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

# The quick-add box's own hx-post endpoint, read from the inbox's markup --
# not assumed.
qa_quick_add_endpoint() {
  local page="$1"
  python3 -c '
import re, sys
m = re.search(r"<form hx-post=\"([^\"]+)\" hx-target=\"#captures\"", sys.argv[1])
print(m.group(1) if m else "")
' "$page"
}

# Submits the quick-add box (form-encoded, exactly as the page's own form
# would) and sets STATUS and BODY -- BODY is the new capture row plus the
# out-of-band suggestions datalist (render_capture_row_response's own
# shape). With one argument, context_tag is omitted from the submission
# entirely; with two, it is sent (an empty second argument sends the field
# present-but-empty) -- the same two distinct "no tag" shapes qa/context_
# tags.md's "a tag is kept, and is optional" procedure asks apart.
qa_quick_add() {
  local raw_text="$1" endpoint response
  endpoint="$(qa_quick_add_endpoint "$(qa_get_inbox)")"
  if (( $# >= 2 )); then
    response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
      -H 'content-type: application/x-www-form-urlencoded' \
      --data-urlencode "raw_text=$raw_text" --data-urlencode "context_tag=$2" --data-urlencode "source=web")"
  else
    response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
      -H 'content-type: application/x-www-form-urlencoded' \
      --data-urlencode "raw_text=$raw_text" --data-urlencode "source=web")"
  fi
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# The capture id a quick-add response's own row carries.
qa_capture_id_from_response() {
  local body="$1"
  python3 -c '
import re, sys
m = re.search(r"<li id=\"capture-row-(\d+)\">", sys.argv[1])
print(m.group(1) if m else "")
' "$body"
}

# Every option value inside the first <datalist id="context-tag-suggestions">
# in text -- one per line, in document order. Matches both the page's own
# datalist (GET /, inside #lists) and the out-of-band variant a quick-add or
# triage response carries; both share the id and the same <option> shape.
qa_tag_suggestions() {
  local text="$1"
  python3 -c '
import re, sys
m = re.search(r"<datalist id=\"context-tag-suggestions\"[^>]*>(.*?)</datalist>", sys.argv[1], re.S)
block = m.group(1) if m else ""
for value in re.findall(r"<option value=\"([^\"]*)\">", block):
    print(value)
' "$text"
}

# The full text of the first <li>...</li> inside <ul id="$2"> containing
# needle, or "" if none matches.
qa_row_line_containing() {
  local page="$1" html_id="$2" needle="$3"
  python3 -c '
import re, sys
page, html_id, needle = sys.argv[1], sys.argv[2], sys.argv[3]
section_m = re.search(r"<ul id=\"" + re.escape(html_id) + r"\">(.*?)</ul>", page, re.S)
section = section_m.group(1) if section_m else ""
for m in re.finditer(r"<li[^>]*>(.*?)</li>", section, re.S):
    if needle in m.group(1):
        print(m.group(1))
        sys.exit()
' "$page" "$html_id" "$needle"
}

qa_capture_line_for() { qa_row_line_containing "$1" captures "$2"; }
qa_task_line_for() { qa_row_line_containing "$1" tasks "$2"; }

# --- Procedure: a tag is kept, and is optional ---
name="tag-is-kept-and-optional"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_quick_add "buy screws" "@homedepot"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a tagged quick-add returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_quick_add "return the drill" ""
  drill_id="$(qa_capture_id_from_response "$BODY")"
  qa_quick_add "pick up milk"
  milk_id="$(qa_capture_id_from_response "$BODY")"

  page="$(qa_get_inbox)"
  screws_line="$(qa_capture_line_for "$page" "buy screws")"
  if [[ "$screws_line" != *"@homedepot"* ]]; then
    echo "FAIL: [$name] expected \"buy screws\" to show @homedepot, got: $screws_line" >&2
    FAILURES=1
  fi

  screws_tag="$(sqlite3 "$DB_PATH" "SELECT IFNULL(context_tag,'NULL') FROM captures WHERE raw_text = 'buy screws';")"
  if [[ "$screws_tag" != "@homedepot" ]]; then
    echo "FAIL: [$name] expected buy screws stored as @homedepot, got $screws_tag" >&2
    FAILURES=1
  fi
  drill_tag="$(sqlite3 "$DB_PATH" "SELECT IFNULL(context_tag,'NULL') FROM captures WHERE id = $drill_id;")"
  milk_tag="$(sqlite3 "$DB_PATH" "SELECT IFNULL(context_tag,'NULL') FROM captures WHERE id = $milk_id;")"
  if [[ "$drill_tag" != "NULL" ]]; then
    echo "FAIL: [$name] an empty context_tag field should store no tag, got $drill_tag" >&2
    FAILURES=1
  fi
  if [[ "$milk_tag" != "NULL" ]]; then
    echo "FAIL: [$name] an omitted context_tag field should store no tag, got $milk_tag" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the suggestions offer every prior tag, once ---
name="suggestions-offer-every-prior-tag-once"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_quick_add "buy screws" "@homedepot"
  qa_quick_add "buy tomatoes" "@supermarket"
  qa_quick_add "buy a hammer" "@homedepot"
  suggestions="$(qa_tag_suggestions "$(qa_get_inbox)")"
  expected="@homedepot
@supermarket"
  if [[ "$suggestions" != "$expected" ]]; then
    echo "FAIL: [$name] expected exactly @homedepot,@supermarket once each, got:
$suggestions" >&2
    FAILURES=1
  fi
  qa_quick_add "no tag here" ""
  suggestions="$(qa_tag_suggestions "$(qa_get_inbox)")"
  if [[ "$suggestions" != "$expected" ]]; then
    echo "FAIL: [$name] an untagged capture should not contribute to the suggestion list, got:
$suggestions" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: case does not split a tag ---
name="case-does-not-split-a-tag"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_quick_add "buy screws" "@HomeDepot"
  qa_quick_add "return the drill" "@homedepot"

  suggestions="$(qa_tag_suggestions "$(qa_get_inbox)")"
  if [[ "$suggestions" != "@HomeDepot" ]]; then
    echo "FAIL: [$name] expected one suggestion spelled @HomeDepot (the first spelling used), got:
$suggestions" >&2
    FAILURES=1
  fi

  page="$(qa_get_inbox)"
  screws_line="$(qa_capture_line_for "$page" "buy screws")"
  drill_line="$(qa_capture_line_for "$page" "return the drill")"
  if [[ "$screws_line" != *"@HomeDepot"* ]]; then
    echo "FAIL: [$name] expected \"buy screws\" to display @HomeDepot, got: $screws_line" >&2
    FAILURES=1
  fi
  if [[ "$drill_line" != *"@HomeDepot"* ]]; then
    echo "FAIL: [$name] expected \"return the drill\" (typed lower-case) to also display @HomeDepot, got: $drill_line" >&2
    FAILURES=1
  fi

  screws_tag="$(sqlite3 "$DB_PATH" "SELECT context_tag FROM captures WHERE raw_text = 'buy screws';")"
  drill_tag="$(sqlite3 "$DB_PATH" "SELECT context_tag FROM captures WHERE raw_text = 'return the drill';")"
  if [[ "$screws_tag" != "@HomeDepot" || "$drill_tag" != "@HomeDepot" ]]; then
    echo "FAIL: [$name] expected both stored rows spelled @HomeDepot, got screws=$screws_tag drill=$drill_tag" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a tag can be given at triage ---
name="tag-given-at-triage"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "buy screws")"
  qa_triage "$capture_id" '{"kind":"pool","context_tag":"@homedepot"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging with a context tag returned status $STATUS" >&2
    FAILURES=1
  fi

  tasks_line="$(qa_task_line_for "$(qa_get_inbox)" "buy screws")"
  if [[ "$tasks_line" != *"@homedepot"* ]]; then
    echo "FAIL: [$name] expected the task list to show @homedepot, got: $tasks_line" >&2
    FAILURES=1
  fi

  capture_tag="$(sqlite3 "$DB_PATH" "SELECT context_tag FROM captures WHERE id = $capture_id;")"
  if [[ "$capture_tag" != "@homedepot" ]]; then
    echo "FAIL: [$name] expected the tag stored on the capture, got $capture_tag" >&2
    FAILURES=1
  fi
  tasks_has_own_column="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name = 'context_tag';")"
  if [[ "$tasks_has_own_column" != "0" ]]; then
    echo "FAIL: [$name] the tag must be stored once, on the capture -- the tasks table should carry no context_tag column of its own" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: triage no longer requires a life area ---
name="triage-no-longer-requires-a-life-area"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  pool_id="$(qa_submit_capture "buy milk")"
  qa_triage "$pool_id" '{"kind":"pool"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a JSON pool triage with no life area returned status $STATUS" >&2
    FAILURES=1
  fi

  committed_id="$(qa_submit_capture "call the dentist")"
  qa_triage "$committed_id" '{"kind":"committed","deadline":"2026-08-25T17:00:00Z","deadline_type":"hard","priority":"P1","estimated_minutes":60}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a JSON committed triage with no life area returned status $STATUS" >&2
    FAILURES=1
  fi

  quota_id="$(qa_submit_capture "go to the gym")"
  qa_triage "$quota_id" '{"kind":"quota","target_count":3,"target_minutes_each":45,"period":"week"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a JSON quota triage with no life area returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_inbox)"
  for text in "buy milk" "call the dentist" "go to the gym"; do
    line="$(qa_task_line_for "$page" "$text")"
    if [[ "$line" == *"("* ]]; then
      echo "FAIL: [$name] expected \"$text\" to carry no life area, got: $line" >&2
      FAILURES=1
    fi
  done

  # Now the page's own form transport, same claim.
  page_pool_id="$(qa_submit_capture "buy screws")"
  pool_row="$(qa_capture_row_block "$(qa_get_inbox)" "$page_pool_id")"
  pool_endpoint="$(qa_block_control_endpoint "$pool_row" 'value="pool"')"
  qa_triage_form "$pool_endpoint" "kind=pool"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a page-form pool triage with no life area returned status $STATUS (body: $BODY)" >&2
    FAILURES=1
  fi
  page="$(qa_get_inbox)"
  line="$(qa_task_line_for "$page" "buy screws")"
  if [[ "$line" == *"("* ]]; then
    echo "FAIL: [$name] expected the page-triaged task to carry no life area, got: $line" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the tags survive a restart ---
name="tags-survive-a-restart"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-1.log"; then
  qa_quick_add "buy screws" "@homedepot"
  qa_quick_add "buy tomatoes" "@supermarket"
  pool_id="$(sqlite3 "$DB" "SELECT id FROM captures WHERE raw_text = 'buy tomatoes';")"
  qa_triage "$pool_id" '{"kind":"pool"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup triage returned status $STATUS" >&2
    FAILURES=1
  fi
  before_page="$(qa_get_inbox)"
  before_captures="$(qa_capture_line_for "$before_page" "buy screws")"
  before_tasks="$(qa_task_line_for "$before_page" "buy tomatoes")"
  before_suggestions="$(qa_tag_suggestions "$before_page")"
  qa_stop_server

  if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-2.log"; then
    after_page="$(qa_get_inbox)"
    after_captures="$(qa_capture_line_for "$after_page" "buy screws")"
    after_tasks="$(qa_task_line_for "$after_page" "buy tomatoes")"
    after_suggestions="$(qa_tag_suggestions "$after_page")"
    if [[ "$after_captures" != "$before_captures" ]]; then
      echo "FAIL: [$name] the inbox row's tag changed across restart: before=[$before_captures] after=[$after_captures]" >&2
      FAILURES=1
    fi
    if [[ "$after_tasks" != "$before_tasks" ]]; then
      echo "FAIL: [$name] the task row's tag changed across restart: before=[$before_tasks] after=[$after_tasks]" >&2
      FAILURES=1
    fi
    if [[ "$after_suggestions" != "$before_suggestions" ]]; then
      echo "FAIL: [$name] the suggestion list changed across restart: before=[$before_suggestions] after=[$after_suggestions]" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile text in a tag stays escaped ---
name="hostile-tag-stays-escaped"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  hostile_tag="<script>alert('boom')</script>"
  qa_quick_add "buy milk" "$hostile_tag"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a hostile-tag quick-add returned status $STATUS" >&2
    FAILURES=1
  fi
  capture_id="$(sqlite3 "$DB_PATH" "SELECT id FROM captures WHERE raw_text = 'buy milk';")"

  page="$(qa_get_inbox)"
  if [[ "$page" == *"<script>alert"* ]]; then
    echo "FAIL: [$name] the inbox page renders an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$page" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive on the inbox page, escaped rather than stripped" >&2
    FAILURES=1
  fi
  suggestions_html="$(python3 -c '
import re, sys
m = re.search(r"<datalist id=\"context-tag-suggestions\"[^>]*>(.*?)</datalist>", sys.argv[1], re.S)
print(m.group(1) if m else "")
' "$page")"
  if [[ "$suggestions_html" == *"<script>alert"* ]]; then
    echo "FAIL: [$name] the suggestion datalist renders an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$suggestions_html" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive in the suggestion datalist -- it renders into an attribute and escapes differently" >&2
    FAILURES=1
  fi

  qa_triage "$capture_id" '{"kind":"pool"}'
  page="$(qa_get_inbox)"
  tasks_section="$(qa_html_section "$page" tasks)"
  if [[ "$tasks_section" == *"<script>alert"* ]]; then
    echo "FAIL: [$name] the task list renders an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$tasks_section" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive in the task list, escaped rather than stripped" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: context_tags"
