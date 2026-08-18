#!/usr/bin/env bash
# Executable QA procedure: qa/app_shell.md (covers features/app_shell.feature).
# Covers the automatable, curl-only procedures from qa/app_shell.md. Drives
# the running server through its HTTP interface only, reading every URL out
# of the header's own markup rather than assuming a route.
#
# qa/app_shell.md's "By-hand walkthrough" is NOT scripted here, and unlike
# every earlier *.sh in this directory, this header does not claim it was
# performed manually this cycle: this environment has no browser-automation
# tooling and no real browser to drive, so steps 3-5 (visited-page identity
# and navigation) and step 8 (a swap leaving the header alone) are covered
# below over curl, which is an honest substitute for those. Steps 6 and 7
# name runtime, visual facts only a real browser shows -- the tab spinner
# turning on a full-page navigation, the back button walking three pages, no
# flicker on a fragment swap. This procedure verifies the static fact that
# implies them (the "the links are ordinary links" procedure below: the
# header carries plain <a href> links with no hx-* attribute, so a browser's
# ordinary full-navigation behaviour applies by construction) rather than
# asserting the runtime behaviour was watched, because it was not.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-app-shell"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_get() {
  curl -s "http://$ADDR$1"
}

# The <header>...</header> region of a rendered page, or "" if not found.
qa_header_of() {
  local page="$1"
  python3 -c '
import sys
page = sys.argv[1]
start = page.find("<header>")
end = page.find("</header>")
print(page[start:end + len("</header>")] if start != -1 and end != -1 else "")
' "$page"
}

# The header's own (label, path, current) triples in document order, as TSV:
# one "label<TAB>path<TAB>current" line per link, current being "1" or "0".
qa_header_links() {
  local header="$1"
  python3 -c '
import re, sys
header = sys.argv[1]
for m in re.finditer(r"<a href=\"([^\"]*)\"([^>]*)>([^<]*)</a>", header):
    path, attrs, label = m.group(1), m.group(2), m.group(3)
    current = "1" if "aria-current=\"page\"" in attrs else "0"
    print(f"{label}\t{path}\t{current}")
' "$header"
}

# --- Procedure: the same header on all three pages ---
name="same-header-on-all-three-pages"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  inbox_header="$(qa_header_of "$(qa_get /)")"
  life_areas_header="$(qa_header_of "$(qa_get /life-areas)")"
  stats_header="$(qa_header_of "$(qa_get /stats)")"

  normalized_inbox="${inbox_header// aria-current=\"page\"/}"
  for pair in "life areas:$life_areas_header" "stats:$stats_header"; do
    label="${pair%%:*}"
    header="${pair#*:}"
    normalized_other="${header// aria-current=\"page\"/}"
    if [[ "$normalized_inbox" != "$normalized_other" ]]; then
      echo "FAIL: [$name] the $label header differs from the inbox's, ignoring only which link is current" >&2
      echo "  inbox:  $normalized_inbox" >&2
      echo "  $label: $normalized_other" >&2
      FAILURES=1
    fi
  done

  links="$(qa_header_links "$inbox_header")"
  labels="$(printf '%s\n' "$links" | cut -f1 | paste -sd, -)"
  if [[ "$labels" != "Inbox,Life areas,Stats" ]]; then
    echo "FAIL: [$name] expected exactly the links Inbox,Life areas,Stats in that order, got: $labels" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the current page is marked, and only it ---
name="current-page-marked-and-only-it"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  declare -A expected=([/]="Inbox" [/life-areas]="Life areas" [/stats]="Stats")
  for path in / /life-areas /stats; do
    header="$(qa_header_of "$(qa_get "$path")")"
    links="$(qa_header_links "$header")"
    current_labels="$(printf '%s\n' "$links" | awk -F'\t' '$3 == "1" {print $1}')"
    current_count="$(printf '%s\n' "$links" | awk -F'\t' '$3 == "1"' | grep -c .)"
    if [[ "$current_count" != "1" ]]; then
      echo "FAIL: [$name] $path marks $current_count entries current, expected exactly 1" >&2
      FAILURES=1
    fi
    if [[ "$current_labels" != "${expected[$path]}" ]]; then
      echo "FAIL: [$name] $path marks \"$current_labels\" current, expected \"${expected[$path]}\"" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: every link goes where it says ---
name="links-go-where-they-say"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  inbox_header="$(qa_header_of "$(qa_get /)")"
  links="$(qa_header_links "$inbox_header")"
  while IFS=$'\t' read -r label path _current; do
    [[ -z "$label" ]] && continue
    header="$(qa_header_of "$(qa_get "$path")")"
    current_labels="$(qa_header_links "$header" | awk -F'\t' '$3 == "1" {print $1}')"
    if [[ "$current_labels" != "$label" ]]; then
      echo "FAIL: [$name] the link labelled \"$label\" leads to $path, which marks \"$current_labels\" current, not \"$label\"" >&2
      FAILURES=1
    fi
  done <<< "$links"
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the 422 swap handling is on every page now, and unchanged where it was ---
name="422-swap-handling-on-every-page"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for path in / /life-areas /stats; do
    page="$(qa_get "$path")"
    if [[ "$page" != *'responseHandling.unshift({code: "422"'* ]]; then
      echo "FAIL: [$name] $path does not declare the 422 swap handling" >&2
      FAILURES=1
    fi
  done

  cid="$(qa_submit_capture "buy milk")"
  qa_triage "$cid" '{"kind":"pool"}'
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] a triage missing life_area should still be rejected with 422, got $STATUS" >&2
    FAILURES=1
  fi

  life_areas_page="$(qa_get /life-areas)"
  add_endpoint="$(qa_life_areas_add_endpoint "$life_areas_page")"
  qa_add_life_area "$add_endpoint" "Work"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] adding a duplicate life area should still be rejected with 422, got $STATUS" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the links are ordinary links ---
name="links-are-ordinary-links"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  header="$(qa_header_of "$(qa_get /)")"
  if [[ "$header" == *"hx-"* ]]; then
    echo "FAIL: [$name] the header carries an hx-* attribute; links should be plain <a href>" >&2
    FAILURES=1
  fi
  anchor_count="$(python3 -c 'import re,sys; print(len(re.findall(r"<a\b", sys.argv[1])))' "$header")"
  href_count="$(python3 -c 'import re,sys; print(len(re.findall(r"<a href=", sys.argv[1])))' "$header")"
  if [[ "$anchor_count" == "0" || "$anchor_count" != "$href_count" ]]; then
    echo "FAIL: [$name] expected every header <a> to be a plain <a href>, found $anchor_count anchors and $href_count with a real href" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a fragment swap leaves the header alone ---
name="fragment-swap-leaves-header-alone"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get /)"

  pool_id="$(qa_submit_capture "buy milk")"
  pool_endpoint="$(qa_row_control_endpoint "$page" "$pool_id" 'value="pool"')"
  qa_triage_form "$pool_endpoint" "kind=pool&life_area=Work"
  if [[ "$BODY" == *"<header>"* ]]; then
    echo "FAIL: [$name] the triage response carries a <header>; it should be the bare #lists fragment" >&2
    FAILURES=1
  fi

  page="$(qa_get /)"
  dismiss_id="$(qa_submit_capture "asdfgh")"
  dismiss_endpoint="$(qa_row_control_endpoint "$page" "$dismiss_id" '>Dismiss<')"
  response="$(curl -s -X POST "http://$ADDR$dismiss_endpoint")"
  if [[ "$response" == *"<header>"* ]]; then
    echo "FAIL: [$name] the dismissal response carries a <header>; it should be the bare #lists fragment" >&2
    FAILURES=1
  fi

  quick_add_response="$(curl -s -X POST "http://$ADDR/captures" \
    -H 'content-type: application/x-www-form-urlencoded' \
    --data-urlencode "raw_text=call the dentist" --data-urlencode "source=web")"
  if [[ "$quick_add_response" == *"<header>"* ]]; then
    echo "FAIL: [$name] the quick-add response carries a <header>; it should be the bare capture row" >&2
    FAILURES=1
  fi

  page="$(qa_get /)"
  header_count="$(python3 -c 'import sys; print(sys.argv[1].count("<header>"))' "$page")"
  if [[ "$header_count" != "1" ]]; then
    echo "FAIL: [$name] expected exactly one <header> on the page afterwards, found $header_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the header renders no user data ---
name="header-renders-no-user-data"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_submit_capture "<script>alert('boom')</script>" >/dev/null
  add_endpoint="$(qa_life_areas_add_endpoint "$(qa_get /life-areas)")"
  qa_add_life_area "$add_endpoint" "<script>alert('boom')</script>"

  for path in / /life-areas /stats; do
    header="$(qa_header_of "$(qa_get "$path")")"
    if [[ "$header" == *"<script>"* ]]; then
      echo "FAIL: [$name] $path's header contains an unescaped <script> tag" >&2
      FAILURES=1
    fi
    if [[ "$header" == *"boom"* ]]; then
      echo "FAIL: [$name] $path's header contains the word boom; the header should render no user-supplied text at all" >&2
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
echo "PASS: app_shell"
