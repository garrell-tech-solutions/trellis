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

# The named gotcha (#114): a systemd --user unit has $HOME, so gh finds
# ~/.config/gh, but it is not a login shell and its environment is the user
# manager's, not an interactive one's. On this host the token is not in
# hosts.yml at all -- it is in the GNOME keyring -- so authentication here
# depends on something outside gh's own config directory. Checked explicitly,
# up front, because the alternative is a `gh pr list` that returns nothing and
# is indistinguishable from "no pull request is labelled preview": the failure
# would look exactly like the quiet, correct, do-nothing path.
if ! AUTH_STATUS="$(gh auth status --hostname github.com 2>&1)"; then
  err "gh is not authenticated from this unit -- polling cannot continue"
  err "$AUTH_STATUS"
  err "gh's token on this host lives in the login keyring, not ~/.config/gh/hosts.yml;"
  err "a user unit running with nobody logged in has no unlocked keyring to read it from."
  exit 1
fi

# One request answers everything this script needs to decide: which pull
# requests carry the label, what commit each one heads at, and whether CI has
# finished on it.
PR_JSON="$(gh pr list \
  --repo "$REPO" \
  --state open \
  --label "$LABEL" \
  --json number,url,headRefName,headRefOid,statusCheckRollup)"

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
debug "statusCheckRollup for #$NUMBER at $SHA: $ROLLUP"

# A rollup entry is either a check run (status/conclusion/name) or a commit
# status (state/context). Read defensively rather than by type: a missing
# `status` reads as finished and a missing `state` as fine, so each kind of
# entry is judged only by the fields it actually has.
CHECKS_TOTAL="$(jq 'length' <<<"$ROLLUP")"
CHECKS_PENDING="$(jq '[.[] | select(((.status // "COMPLETED") != "COMPLETED") or ((.state // "SUCCESS") | IN("PENDING","EXPECTED")))] | length' <<<"$ROLLUP")"
CHECKS_BAD="$(jq -r '[.[] | select((((.conclusion // "SUCCESS") | IN("SUCCESS","NEUTRAL","SKIPPED")) | not) or ((.state // "SUCCESS") | IN("FAILURE","ERROR"))) | (.name // .context // "check")] | join(", ")' <<<"$ROLLUP")"

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

if ! "$PREVIEW_SH" "$BRANCH"; then
  err "ops/preview.sh failed for pull request #$NUMBER ($BRANCH) at $SHA -- see the preceding lines"
  err "the preview has NOT been updated; the next tick will try again"
  exit 1
fi

# Only now. Recording the SHA before the run would mean one failure silently
# retires that commit: the next tick would see it as already previewed and do
# nothing, forever, with :8443 still serving whatever it served before.
printf '%s' "$SHA" >"$SHA_FILE"
info "preview is now pull request #$NUMBER ($BRANCH) at $SHA"
