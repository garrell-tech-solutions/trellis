#!/usr/bin/env bash
# ops/preview-poll.sh
#
# What invokes ops/preview.sh. #102 built the preview machinery and never
# said what triggers it, so nothing did: the preview served `ops/branch-preview`
# for a day after that branch merged, and #105 and #113 both went to review
# unpreviewed (issue #114).
#
# Outbound poll, on a systemd user timer (ops/trellis-preview-poll.timer).
# The box asks GitHub; GitHub never reaches in. R-multi-tenancy makes the
# tailnet the entire security boundary, so there is no webhook to receive, no
# port to open and no self-hosted runner -- a fork's pull request executing on
# the host that holds the owner's live database is the hole #104 closed, and a
# runner here would reopen it wider.
#
# Selection is by label, because of where the owner is standing when they want
# a preview: away from the desk, with the GitHub app already open on the phone
# that is going to display it. Tapping `preview` is one gesture from there.
# (Rejected in #114: previewing the most recently updated open pull request.
# One slot plus automatic selection means a second branch yanks the page out
# from under whoever is reading the first.)
#
# This script decides WHICH branch and WHETHER to act. It decides nothing about
# what a preview is: ops/preview.sh is invoked unchanged and still owns the
# artifact selection, the fork-artifact trust guard, the static-binary check,
# the wipe-and-re-migrate and the reseed. In particular the fork refusal is
# not repeated here -- repeating it would mean two copies of a security
# decision that must agree, so a labelled pull request from a fork reaches
# ops/preview.sh and is refused there, loudly, exactly as it would be by hand.
#
# Three things this has to get right, all of them from #114's acceptance:
#
#   Idempotence.  ops/preview.sh wipes preview.db and restarts the unit every
#                 time it runs. So an unchanged head SHA must do NOTHING --
#                 not a re-download, not a reseed, not a restart. A timer that
#                 rebuilds the preview every two minutes destroys whatever the
#                 owner was mid-way through looking at, which is worse than
#                 the manual trigger it replaces. The last SHA successfully
#                 previewed is remembered in $STATE_DIR, and written only
#                 after ops/preview.sh has actually succeeded.
#
#   Not guessing. Zero labelled pull requests leaves the running preview alone
#                 and says so. Two or more is refused by name, never resolved
#                 by picking the newest -- there is one slot, and which branch
#                 the owner meant is not this script's to invent.
#
#   Quiet.        A labelled pull request whose CI has not finished is the
#                 normal state for the first minutes after it opens, not a
#                 failure. It is announced once and then waited on in silence
#                 (see `note` below) until something actually changes.
set -euo pipefail

REPO="garrell-tech-solutions/trellis"
LABEL="preview"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREVIEW_SH="$SCRIPT_DIR/preview.sh"

# State, not cache: losing it costs one needless preview rebuild, which is why
# it lives under $XDG_STATE_HOME rather than in a cache directory something is
# entitled to clear. Deleting preview-poll.sha is also the supported way to
# force the next tick to re-preview the labelled branch -- for instance after
# someone has run `ops/preview.sh` by hand against something else, which this
# script cannot see and does not try to.
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/trellis"
SHA_FILE="$STATE_DIR/preview-poll.sha"
NOTE_FILE="$STATE_DIR/preview-poll.note"

mkdir -p "$STATE_DIR"

# Journal priorities. systemd's SyslogLevelPrefix (on by default) reads a
# leading `<N>` off each line and files the message at that priority, so
# `journalctl --user -u trellis-preview-poll -p warning` shows the things that
# need a human and hides the ticking. Under a terminal there is no journal to
# file anything into, so the prefix would just be noise -- $JOURNAL_STREAM is
# set by systemd exactly when stdout is the journal.
log() {
  local prio="$1"; shift
  if [[ -n "${JOURNAL_STREAM:-}" ]]; then
    printf '<%s>%s\n' "$prio" "$*" >&2
  else
    printf '%s\n' "$*" >&2
  fi
}
err()   { log 3 "$@"; }
warn()  { log 4 "$@"; }
info()  { log 6 "$@"; }
debug() { log 7 "$@"; }

# note <priority> <message> -- say it the first time, then stop saying it.
#
# Every routine "nothing to do here" message goes through this. Polling every
# two minutes means 720 ticks a day, and a state that persists (no labelled
# pull request; CI still running) would otherwise repeat its line 720 times
# and bury the one line that mattered. Repeats are still recorded, at debug,
# so `journalctl -p debug` can still prove the timer was alive.
note() {
  local prio="$1" msg="$2" last=""
  [[ -f "$NOTE_FILE" ]] && last="$(cat "$NOTE_FILE")"
  if [[ "$msg" == "$last" ]]; then
    debug "(unchanged) $msg"
  else
    log "$prio" "$msg"
    printf '%s' "$msg" >"$NOTE_FILE"
  fi
}

command -v gh >/dev/null || {
  err "gh (GitHub CLI) is required and not on PATH: PATH=$PATH"
  exit 1
}
command -v jq >/dev/null || {
  err "jq is required and not on PATH: PATH=$PATH"
  exit 1
}
[[ -x "$PREVIEW_SH" ]] || {
  err "$PREVIEW_SH is missing or not executable -- this script must sit beside ops/preview.sh"
  exit 1
}

# One request answers everything this script needs to decide: which pull
# requests carry the label, what commit each one heads at, and whether CI has
# finished on it.
#
# Every open pull request, filtered here -- not `gh pr list --label`. That flag
# is served by GitHub's search index, which lags the label: adding `preview`
# to #115 and immediately asking for it by label returned nothing for about
# five seconds, while the label was already on the pull request object. A
# two-minute poll would have shrugged that off, but the failure it produces is
# the bad one -- an empty list is indistinguishable from "nothing is labelled",
# so a lagging index reads as the owner not having asked for a preview, and if
# the lag ever outlasts a tick the log would say the wrong thing confidently.
# The labels on the pull request objects themselves are not indexed, they are
# the record, and this repo has a handful of open pull requests, so filtering
# them here costs one request either way.
# The named gotcha (#114): a systemd --user unit has $HOME, so gh finds
# ~/.config/gh, but it is not a login shell and its environment is the user
# manager's rather than an interactive one's. On this host the token is not in
# hosts.yml at all -- hosts.yml names the account and nothing else, and the
# token itself is in the login keyring -- so whether gh can authenticate from
# here depends on something outside gh's own config directory entirely.
#
# So the request is made, and its failure is what gets checked. NOT
# `gh auth status`: that exits 0 while the ACTIVE account's token is invalid,
# as long as some other account in the keyring still works. Observed here --
# with a deliberately bogus GH_TOKEN it printed
#
#   X Failed to log in to github.com using token (GH_TOKEN)
#   - The token in GH_TOKEN is invalid.
#
# and returned 0, and the script sailed past its own preflight into a 401 on
# the next line. A preflight that cannot fail is worse than no preflight,
# because it is read as evidence (decisions.md T-a-check-must-be-seen-to-fail).
# The listing below is the request whose success actually matters, so it is
# the one whose failure is reported.
GH_OUT="$(mktemp)"
GH_ERR="$(mktemp)"
trap 'rm -f "$GH_OUT" "$GH_ERR"' EXIT

if ! gh pr list \
  --repo "$REPO" \
  --state open \
  --limit 100 \
  --json number,url,headRefName,headRefOid,labels,statusCheckRollup \
  >"$GH_OUT" 2>"$GH_ERR"; then
  err "could not list open pull requests on $REPO -- polling cannot continue"
  while IFS= read -r line; do err "  gh: $line"; done <"$GH_ERR"
  err "if that is an authentication failure: gh's token on this host is in the login"
  err "keyring, not ~/.config/gh/hosts.yml, and a user unit running with nobody"
  err "logged in has no unlocked keyring to read it from."
  exit 1
fi
ALL_JSON="$(cat "$GH_OUT")"

PR_JSON="$(jq -c --arg label "$LABEL" '[.[] | select(.labels | any(.name == $label))]' <<<"$ALL_JSON")"

COUNT="$(jq 'length' <<<"$PR_JSON")"

if [[ "$COUNT" -eq 0 ]]; then
  note 6 "no open pull request is labelled '$LABEL'; leaving the running preview as it is"
  exit 0
fi

if [[ "$COUNT" -gt 1 ]]; then
  # Named, all of them, and refused. There is one preview slot and no basis in
  # here for choosing between them; picking the newest would silently take the
  # page away from whichever one the owner was reading.
  NAMES="$(jq -r '[.[] | "#\(.number) (\(.headRefName))"] | join(", ")' <<<"$PR_JSON")"
  err "refusing to preview: $COUNT open pull requests are labelled '$LABEL': $NAMES"
  err "there is one preview slot; remove '$LABEL' from all but the one you want"
  exit 1
fi

NUMBER="$(jq -r '.[0].number' <<<"$PR_JSON")"
BRANCH="$(jq -r '.[0].headRefName' <<<"$PR_JSON")"
SHA="$(jq -r '.[0].headRefOid' <<<"$PR_JSON")"

# Idempotence, before anything else is considered. Same commit as the last
# successful run means the preview already on :8443 IS this pull request, and
# the correct amount of work to do is none.
LAST_SHA=""
[[ -f "$SHA_FILE" ]] && LAST_SHA="$(cat "$SHA_FILE")"
if [[ "$SHA" == "$LAST_SHA" ]]; then
  debug "pull request #$NUMBER ($BRANCH) is already previewed at $SHA; nothing to do"
  exit 0
fi

# Is CI finished on this commit, and green?
#
# This asks the pull request whether it is green. It does not go looking for
# artifacts or runs -- ops/preview.sh owns that, and pins its choice to
# headRefOid through the pull request rather than to a branch name. The only
# thing being decided here is whether to invoke it yet or wait for the next
# tick, because a pull request opened thirty seconds ago has no successful run
# for ops/preview.sh to find and that is not a fault to report.
ROLLUP="$(jq -c '.[0].statusCheckRollup // []' <<<"$PR_JSON")"

# A rollup entry is either a check run (name/status/conclusion) or a commit
# status (context/state), and the two are read here without asking which is
# which: a field the entry does not have is absent, and a field it has but has
# not filled in yet is the EMPTY STRING, not null. That distinction is the
# whole reason this normalises first. `.conclusion // "SUCCESS"` looks right
# and is wrong, because `//` only substitutes for null and false -- an
# in-progress check run reports `"conclusion": ""`, which sails past the
# default and then fails the "is it a success" test, marking every running
# check as failed. Observed on #115's own first tick.
NORM="$(jq -c '[.[] | {
    name:       (.name // .context // "check"),
    status:     (if (.status // "") == "" then "COMPLETED" else .status end),
    conclusion: (if (.conclusion // "") == "" then null else .conclusion end),
    state:      (if (.state // "") == "" then null else .state end)
  }]' <<<"$ROLLUP")"
debug "checks for #$NUMBER at $SHA: $NORM"

CHECKS_TOTAL="$(jq 'length' <<<"$NORM")"
CHECKS_PENDING="$(jq '[.[] | select(.status != "COMPLETED" or (.state != null and (.state | IN("PENDING","EXPECTED"))))] | length' <<<"$NORM")"
# Only entries that have finished can be judged to have failed, so a running
# check can never be counted here whatever it reports in the meantime.
CHECKS_BAD="$(jq -r '[.[]
    | select(.status == "COMPLETED")
    | select((.conclusion != null and ((.conclusion | IN("SUCCESS","NEUTRAL","SKIPPED")) | not))
             or (.state != null and (.state | IN("FAILURE","ERROR"))))
    | .name] | join(", ")' <<<"$NORM")"

if [[ "$CHECKS_TOTAL" -eq 0 ]]; then
  note 6 "pull request #$NUMBER ($BRANCH) at $SHA has reported no checks yet; waiting"
  exit 0
fi
if [[ "$CHECKS_PENDING" -gt 0 ]]; then
  note 6 "pull request #$NUMBER ($BRANCH) at $SHA is still running CI ($CHECKS_PENDING check(s) outstanding); waiting"
  exit 0
fi
if [[ -n "$CHECKS_BAD" ]]; then
  # Not an error of this script's, and not something retrying will fix -- but
  # the owner labelled it expecting a preview and is not getting one, so it is
  # said once, at warning, rather than dropped.
  note 4 "pull request #$NUMBER ($BRANCH) at $SHA has finished CI without succeeding (failing: $CHECKS_BAD); not previewing"
  exit 0
fi

info "previewing pull request #$NUMBER ($BRANCH) at $SHA"

# Cleared before, not after: the run below replaces the preview, so whatever
# routine state was last announced is over regardless of how this ends.
rm -f "$NOTE_FILE"

# Output is streamed rather than captured and re-emitted, so a run that takes
# a minute shows its progress while it is happening rather than all at once
# when it is over.
#
# One caveat, observed rather than assumed: journald occasionally fails to
# attribute a line to this unit when the process that wrote it exits within a
# few milliseconds. A stub standing in for a fork refusal was logged twice in
# identical runs -- once attributed and once not, the unattributed one still
# present in the unfiltered journal at the same millisecond. The real
# ops/preview.sh lives for seconds, so this does not bite it, and this
# script's own lines always come from the long-running parent and are always
# attributed. The message below says where else to look rather than promising
# the detail is directly above it.
if ! "$PREVIEW_SH" "$BRANCH"; then
  err "ops/preview.sh failed for pull request #$NUMBER ($BRANCH) at $SHA"
  err "its own explanation is above; if journald did not attribute it to this unit,"
  err "it is in \`journalctl --user --since '10 min ago'\` unfiltered"
  err "the preview has NOT been updated; the next tick will try again"
  exit 1
fi

# Only now. Recording the SHA before the run would mean one failure silently
# retires that commit: the next tick would see it as already previewed and do
# nothing, forever, with :8443 still serving whatever it served before.
printf '%s' "$SHA" >"$SHA_FILE"
info "preview is now pull request #$NUMBER ($BRANCH) at $SHA"
