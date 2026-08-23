#!/usr/bin/env bash
# Executable QA procedure: qa/installable.md (covers
# features/installable.feature). Drives the running server through its
# HTTP interface only -- the three screens, the manifest link they carry,
# and whatever the manifest itself names -- and inspects nothing beyond
# what the page and the manifest actually say.
#
# T-qa-binds-tolerantly-to-markup governs the extraction here exactly as
# qa/installable.md and crates/acceptance-tests/src/steps/installable.rs
# both insist: the manifest link's href and each icon's src are read out
# of the page/manifest, never assumed at a fixed path.
#
# WHAT THIS SCRIPT CANNOT CHECK, per qa/installable.md's own "What can be
# checked, and what cannot": whether a real browser offers to install, and
# how an installed standalone window behaves (no address bar, tab bar
# above the safe area). Both are by-hand-only; qa/installable.md's
# walkthrough exists for exactly that reason and is not scripted here.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-installable"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

SCREENS=(/ /pool /committed)

qa_get_screen() {
  curl -s "http://$ADDR$1"
}

# The href a <link rel="manifest" href="..."> names in body, or "" if
# absent -- read out of the page rather than assumed, matching
# installable.rs's own manifest_href.
qa_manifest_href() {
  python3 -c '
import re, sys
m = re.search(r"<link rel=\"manifest\" href=\"([^\"]+)\">", sys.argv[1])
print(m.group(1) if m else "")
' "$1"
}

# The content a <meta name="theme-color" content="..."> names in body, or
# "" if absent.
qa_theme_colour() {
  python3 -c '
import re, sys
m = re.search(r"<meta name=\"theme-color\" content=\"([^\"]+)\">", sys.argv[1])
print(m.group(1) if m else "")
' "$1"
}

if qa_start_server "$BIN" "$TMP_DIR/installable.sqlite" "$TMP_DIR/installable.log"; then

  # --- Procedure: the manifest is linked and served ---
  name="the-manifest-is-linked-and-served"
  declare -a hrefs=()
  for screen in "${SCREENS[@]}"; do
    body="$(qa_get_screen "$screen")"
    href="$(qa_manifest_href "$body")"
    if [[ -z "$href" ]]; then
      echo "FAIL: [$name] $screen carries no manifest <link>" >&2
      FAILURES=1
    fi
    hrefs+=("$href")
  done
  if [[ "${hrefs[0]}" != "${hrefs[1]}" || "${hrefs[1]}" != "${hrefs[2]}" ]]; then
    echo "FAIL: [$name] the three screens link different manifests: ${hrefs[*]}" >&2
    FAILURES=1
  fi
  manifest_href="${hrefs[0]}"

  manifest_status="$(curl -s -o "$TMP_DIR/manifest.json" -w '%{http_code}' "http://$ADDR$manifest_href")"
  if [[ "$manifest_status" != "200" ]]; then
    echo "FAIL: [$name] fetching the manifest at $manifest_href returned status $manifest_status" >&2
    FAILURES=1
  fi
  manifest_body="$(cat "$TMP_DIR/manifest.json")"
  manifest_parsed=1
  if ! echo "$manifest_body" | python3 -c 'import json,sys; json.load(sys.stdin)' 2>"$TMP_DIR/manifest-parse-error.log"; then
    echo "FAIL: [$name] the manifest at $manifest_href did not parse as JSON:" >&2
    cat "$TMP_DIR/manifest-parse-error.log" >&2
    echo "body:" >&2
    echo "$manifest_body" >&2
    FAILURES=1
    manifest_parsed=0
  fi

if [[ "$manifest_parsed" == "1" ]]; then
  # --- Procedure: the manifest carries the members an installable app requires ---
  name="required-members"
  for pair in "name=Trellis" "short_name=Trellis" "start_url=/" "display=standalone"; do
    member="${pair%%=*}"
    expected="${pair#*=}"
    actual="$(python3 -c '
import json, sys
manifest, member = sys.argv[1], sys.argv[2]
print(json.loads(manifest).get(member, ""))
' "$manifest_body" "$member")"
    if [[ "$actual" != "$expected" ]]; then
      echo "FAIL: [$name] expected manifest member \"$member\" to be \"$expected\", got \"$actual\"" >&2
      FAILURES=1
    fi
  done

  # --- Procedure: every icon resolves ---
  name="every-icon-resolves"
  icon_rows="$(python3 -c '
import json, sys
for icon in json.loads(sys.argv[1]).get("icons", []):
    fields = [icon.get("src", ""), icon.get("sizes", ""), icon.get("type", ""), icon.get("purpose", "")]
    print("\t".join(fields))
' "$manifest_body")"
  if [[ -z "$icon_rows" ]]; then
    echo "FAIL: [$name] the manifest declares no icons" >&2
    FAILURES=1
  fi

  found_192=0
  found_512=0
  found_maskable=0
  while IFS=$'\t' read -r src sizes type purpose; do
    [[ -z "$src" ]] && continue
    [[ "$sizes" == "192x192" ]] && found_192=1
    [[ "$sizes" == "512x512" && "$purpose" != *maskable* ]] && found_512=1
    [[ "$purpose" == *maskable* ]] && found_maskable=1

    icon_status="$(curl -s -o "$TMP_DIR/icon-fetch.png" -w '%{http_code}' "http://$ADDR$src")"
    if [[ "$icon_status" != "200" ]]; then
      echo "FAIL: [$name] icon $src returned status $icon_status" >&2
      FAILURES=1
      continue
    fi
    icon_content_type="$( (curl -s -D - -o /dev/null "http://$ADDR$src" | tr -d '\r' | grep -i '^content-type:' | cut -d' ' -f2-) || true)"
    if [[ "$icon_content_type" != "$type" ]]; then
      echo "FAIL: [$name] icon $src declares type \"$type\" but served content-type \"$icon_content_type\"" >&2
      FAILURES=1
    fi
    magic="$(head -c4 "$TMP_DIR/icon-fetch.png" | od -An -tx1 | tr -d ' \n')"
    if [[ "$magic" != "89504e47" ]]; then
      echo "FAIL: [$name] icon $src claims $type but its bytes are not a PNG signature (got $magic)" >&2
      FAILURES=1
    fi
  done <<<"$icon_rows"

  if [[ "$found_192" != "1" ]]; then
    echo "FAIL: [$name] no 192x192 icon declared" >&2
    FAILURES=1
  fi
  if [[ "$found_512" != "1" ]]; then
    echo "FAIL: [$name] no 512x512 (non-maskable) icon declared" >&2
    FAILURES=1
  fi
  if [[ "$found_maskable" != "1" ]]; then
    echo "FAIL: [$name] no icon with purpose \"maskable\" declared" >&2
    FAILURES=1
  fi

  # --- Procedure: every screen declares a theme colour matching the manifest ---
  name="theme-colour-matches"
  manifest_theme_color="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("theme_color",""))' "$manifest_body")"
  if [[ -z "$manifest_theme_color" ]]; then
    echo "FAIL: [$name] the manifest has no theme_color member" >&2
    FAILURES=1
  fi
  for screen in "${SCREENS[@]}"; do
    body="$(qa_get_screen "$screen")"
    page_colour="$(qa_theme_colour "$body")"
    if [[ -z "$page_colour" ]]; then
      echo "FAIL: [$name] $screen carries no theme-color <meta>" >&2
      FAILURES=1
    elif [[ "$page_colour" != "$manifest_theme_color" ]]; then
      echo "FAIL: [$name] $screen's theme colour \"$page_colour\" does not match the manifest's \"$manifest_theme_color\"" >&2
      FAILURES=1
    fi
  done
else
  echo "SKIP: required-members, every-icon-resolves, theme-colour-matches -- the manifest did not parse" >&2
fi

  # --- Procedure: no service worker ---
  name="no-service-worker"
  for screen in "${SCREENS[@]}"; do
    body="$(qa_get_screen "$screen")"
    if [[ "$body" == *serviceWorker* ]]; then
      echo "FAIL: [$name] $screen registers a service worker" >&2
      FAILURES=1
    fi
  done
  for path in /service-worker.js /sw.js; do
    sw_status="$(curl -s -o /dev/null -w '%{http_code}' "http://$ADDR$path")"
    if [[ "$sw_status" == "200" ]]; then
      echo "FAIL: [$name] $path served 200 -- no service worker script should exist" >&2
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
echo "PASS: installable"
