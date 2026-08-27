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
# prior tag, exactly once, in a datalist the browser then narrows -- plus
# the one static markup fact the walkthrough names that curl can still
# confirm (raw_text keeps focus first, per the "stays one-handed" note).
#
# This is context-tags re-implemented against #88's demolished tree
# (life_areas is gone; pool triage is a single field now). The doc's own
# "nothing else changed" procedure names PR #87's drift as a cautionary
# tale: a template class attribute broke every script's row-matching regex
# once already, silently. capture_row.html is touched again here.
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
m = re.search(r"<form[^>]*hx-post=\"([^\"]+)\"[^>]*hx-target=\"#captures\"", sys.argv[1])
print(m.group(1) if m else "")
' "$page"
}

# Submits the quick-add box (form-encoded, exactly as the page's own form
# would) and sets STATUS and BODY -- BODY is the new capture row plus the
# out-of-band suggestions datalist (render_capture_row_response's own
# shape). With one argument, context_tag is omitted from the submission
# entirely; with two, it is sent (an empty second argument sends the field
# present-but-empty) -- two of the three distinct "no tag" shapes qa/
# context_tags.md's "a tag is kept, and is optional" procedure asks apart
# (the third, whitespace-only, is just a value like any other).
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
m = re.search(r"<li id=\"capture-row-(\d+)\"[^>]*>", sys.argv[1])
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

# #140 retired the flat Tasks list; a triaged pool task's tag now shows on
# the Pool screen (loose end or trip row, per its own count threshold), and
# on the triaged capture's own restyled Recent row.
qa_get_pool() {
  curl -s "http://$ADDR/pool"
}

qa_pool_line_for() {
  local page="$1" needle="$2" loose
  loose="$(qa_loose_section "$page")"
  qa_loose_row_containing "$loose" "$needle"
}

# Elapsed milliseconds of a JSON capture POST, and its status, as
# "status ms". Used to confirm the 50ms budget's subject did not move: the
# suggestion query belongs to the page render, not to this endpoint.
qa_timed_capture() {
  local raw_text="$1" result status time_total time_ms
  result="$(curl -s -o /dev/null -w '%{http_code} %{time_total}' \
    -X POST "http://$ADDR/captures" \
    -H 'content-type: application/json' \
    -d "$(python3 -c 'import json,sys; print(json.dumps({"raw_text": sys.argv[1], "source": "web"}))' "$raw_text")")"
  status="${result%% *}"
  time_total="${result#* }"
  time_ms="$(awk -v t="$time_total" 'BEGIN { printf "%.1f", t * 1000 }')"
  echo "$status $time_ms"
}

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
  qa_quick_add "renew the passport" "   "
  passport_id="$(qa_capture_id_from_response "$BODY")"

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
  for pair in "empty:$drill_id" "omitted:$milk_id" "whitespace-only:$passport_id"; do
    kind="${pair%%:*}" id="${pair#*:}"
    tag="$(sqlite3 "$DB_PATH" "SELECT IFNULL(context_tag,'NULL') FROM captures WHERE id = $id;")"
    if [[ "$tag" != "NULL" ]]; then
      echo "FAIL: [$name] a $kind context_tag should store no tag, got $tag" >&2
      FAILURES=1
    fi
  done

  # Same "no tag" shapes over the JSON transport
  # (T-required-fields-are-specified-per-transport): omitted, empty, and
  # whitespace-only all store no tag, and none is rejected.
  for pair in 'call the dentist:{"raw_text":"call the dentist","source":"web"}' \
              'go to the gym:{"raw_text":"go to the gym","source":"web","context_tag":""}' \
              'water the plants:{"raw_text":"water the plants","source":"web","context_tag":"   "}'; do
    text="${pair%%:*}" json="${pair#*:}"
    json_status="$(curl -s -o /dev/null -w '%{http_code}' -X POST "http://$ADDR/captures" \
      -H 'content-type: application/json' -d "$json")"
    if [[ "$json_status" != "201" ]]; then
      echo "FAIL: [$name] a JSON capture \"$text\" with no real tag returned status $json_status" >&2
      FAILURES=1
    fi
    stored="$(sqlite3 "$DB_PATH" "SELECT IFNULL(context_tag,'NULL') FROM captures WHERE raw_text = '$text';")"
    if [[ "$stored" != "NULL" ]]; then
      echo "FAIL: [$name] a JSON capture \"$text\" with no real tag should store none, got $stored" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the suggestions offer every prior tag, once ---
name="suggestions-offer-every-prior-tag-once"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  read -r before_status before_ms <<< "$(qa_timed_capture "warm-up before tags")"
  if [[ "$before_status" != "201" || "$(awk -v t="$before_ms" 'BEGIN{print (t<50)?1:0}')" != "1" ]]; then
    echo "FAIL: [$name] the capture endpoint before populating tags took ${before_ms}ms (status $before_status), expected under 50ms" >&2
    FAILURES=1
  fi

  qa_quick_add "buy screws" "@homedepot"
  qa_quick_add "buy tomatoes" "@supermarket"
  qa_quick_add "buy a hammer" "@homedepot"
  for i in $(seq 1 20); do
    qa_quick_add "filler item $i" "@tag-$i"
  done
  suggestions="$(qa_tag_suggestions "$(qa_get_inbox)")"
  expected="@homedepot
@supermarket"
  suggestions_head="$(printf '%s\n' "$suggestions" | head -2)"
  if [[ "$suggestions_head" != "$expected" ]]; then
    echo "FAIL: [$name] expected @homedepot,@supermarket first (in first-use order), got:
$suggestions_head" >&2
    FAILURES=1
  fi
  suggestion_count="$(printf '%s\n' "$suggestions" | grep -c .)"
  if [[ "$suggestion_count" != "22" ]]; then
    echo "FAIL: [$name] expected 22 distinct tags (2 shared + 20 filler), got $suggestion_count" >&2
    FAILURES=1
  fi

  qa_quick_add "no tag here" ""
  suggestion_count_after="$(printf '%s\n' "$(qa_tag_suggestions "$(qa_get_inbox)")" | grep -c .)"
  if [[ "$suggestion_count_after" != "22" ]]; then
    echo "FAIL: [$name] an untagged capture should not contribute to the suggestion list, count changed to $suggestion_count_after" >&2
    FAILURES=1
  fi

  # T-latency-is-a-qa-assertion's subject must not move: the suggestion set
  # now has 22 entries, and the capture endpoint's own timing must not have
  # picked up that cost.
  read -r after_status after_ms <<< "$(qa_timed_capture "warm-up after tags")"
  if [[ "$after_status" != "201" || "$(awk -v t="$after_ms" 'BEGIN{print (t<50)?1:0}')" != "1" ]]; then
    echo "FAIL: [$name] the capture endpoint after populating 22 tags took ${after_ms}ms (status $after_status), expected under 50ms -- the suggestion query may have leaked into the capture path" >&2
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

# --- Procedure: a tag can be given at triage, and survives it ---
name="tag-given-at-triage-and-survives-it"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  # Tagged at capture, triaged with no further tag on the triage control --
  # the tag already on the capture must survive the triage untouched.
  qa_quick_add "buy screws" "@homedepot"
  screws_id="$(qa_capture_id_from_response "$BODY")"
  qa_triage "$screws_id" '{"kind":"pool"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the already-tagged capture returned status $STATUS" >&2
    FAILURES=1
  fi

  # Untagged at capture, tagged at triage -- the second chance.
  drill_id="$(qa_submit_capture "return the drill")"
  qa_triage "$drill_id" '{"kind":"pool","context_tag":"@homedepot"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging with a context tag returned status $STATUS" >&2
    FAILURES=1
  fi

  pool_page="$(qa_get_pool)"
  for text in "buy screws" "return the drill"; do
    pool_line="$(qa_pool_line_for "$pool_page" "$text")"
    if [[ "$pool_line" != *"@homedepot"* ]]; then
      echo "FAIL: [$name] expected the Pool screen to show @homedepot for \"$text\", got: $pool_line" >&2
      FAILURES=1
    fi
  done
  # Both restyled Recent rows read their kind and tag too (#140).
  page="$(qa_get_inbox)"
  for text in "buy screws" "return the drill"; do
    recent_line="$(qa_capture_line_for "$page" "$text")"
    if [[ "$recent_line" != *"@homedepot"* ]]; then
      echo "FAIL: [$name] expected the Recent row for \"$text\" to show @homedepot, got: $recent_line" >&2
      FAILURES=1
    fi
  done

  for pair in "screws:$screws_id" "drill:$drill_id"; do
    label="${pair%%:*}" id="${pair#*:}"
    capture_tag="$(sqlite3 "$DB_PATH" "SELECT context_tag FROM captures WHERE id = $id;")"
    if [[ "$capture_tag" != "@homedepot" ]]; then
      echo "FAIL: [$name] expected the $label capture's own tag stored as @homedepot, got $capture_tag" >&2
      FAILURES=1
    fi
  done
  tasks_has_own_column="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name = 'context_tag';")"
  if [[ "$tasks_has_own_column" != "0" ]]; then
    echo "FAIL: [$name] the tag must be stored once, on the capture -- the tasks table should carry no context_tag column of its own" >&2
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
  before_tasks="$(qa_pool_line_for "$(qa_get_pool)" "buy tomatoes")"
  before_suggestions="$(qa_tag_suggestions "$before_page")"
  qa_stop_server

  if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-2.log"; then
    after_page="$(qa_get_inbox)"
    after_captures="$(qa_capture_line_for "$after_page" "buy screws")"
    after_tasks="$(qa_pool_line_for "$(qa_get_pool)" "buy tomatoes")"
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
  pool_page="$(qa_get_pool)"
  if [[ "$pool_page" == *"<script>alert"* ]]; then
    echo "FAIL: [$name] the Pool screen renders an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$pool_page" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive on the Pool screen, escaped rather than stripped" >&2
    FAILURES=1
  fi
  recent_page="$(qa_get_inbox)"
  recent_line="$(qa_capture_line_for "$recent_page" "buy milk")"
  if [[ "$recent_line" == *"<script>alert"* ]]; then
    echo "FAIL: [$name] the restyled Recent row renders an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$recent_line" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive on the restyled Recent row, escaped rather than stripped" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the quick-add box stays one-handed ---
# From the by-hand walkthrough's own outcomes ("the raw-text field still
# takes focus first") -- not a numbered procedure, but a static markup fact
# curl can confirm without a browser: raw_text carries autofocus, the tag
# field does not.
name="quick-add-stays-one-handed"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_inbox)"
  quick_add_form="$(python3 -c '
import re, sys
m = re.search(r"<form class=\"quick-add\"[^>]*>.*?</form>", sys.argv[1], re.S)
print(m.group(0) if m else "")
' "$page")"
  raw_text_input="$(python3 -c '
import re, sys
m = re.search(r"<input[^>]*name=\"raw_text\"[^>]*>", sys.argv[1])
print(m.group(0) if m else "")
' "$quick_add_form")"
  tag_input="$(python3 -c '
import re, sys
m = re.search(r"<input[^>]*name=\"context_tag\"[^>]*>", sys.argv[1])
print(m.group(0) if m else "")
' "$quick_add_form")"
  if [[ "$raw_text_input" != *"autofocus"* ]]; then
    echo "FAIL: [$name] expected the raw_text input to carry autofocus, got: $raw_text_input" >&2
    FAILURES=1
  fi
  if [[ "$tag_input" == *"autofocus"* ]]; then
    echo "FAIL: [$name] the tag field should not compete for focus with raw_text, got: $tag_input" >&2
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
