#!/usr/bin/env bash
# One-time bootstrap for the preview trigger (issue #114): install the poll
# unit and its timer, and start the timer.
#
# Separate from ops/install.sh on purpose. install.sh bootstraps the live
# instance -- trellis.service, ~/.local/bin/trellis, trellis.db -- and #114
# is explicitly not allowed near any of those. Keeping the two installers
# apart means running this one cannot restart the thing the owner uses.
#
# Idempotent: re-running reinstalls the unit files, reloads and re-enables,
# and does not disturb a preview that is already up.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UNIT_DIR="$HOME/.config/systemd/user"
SERVICE="trellis-preview-poll.service"
TIMER="trellis-preview-poll.timer"

mkdir -p "$UNIT_DIR"
install -m 644 "$SCRIPT_DIR/$SERVICE" "$UNIT_DIR/$SERVICE"
install -m 644 "$SCRIPT_DIR/$TIMER" "$UNIT_DIR/$TIMER"

# The unit names one checkout, because the poll script, preview.sh,
# preview-seed.sh and trellis-preview.service have to be the same version of
# each other (see the unit file). If this installer is being run from a
# different checkout than the one the unit names -- a worktree, or a clone
# somewhere else -- then the unit as shipped would silently run some other
# copy, so the path is corrected here rather than left to be discovered later.
#
# Read out of the unit file rather than repeated, so there is still exactly
# one place the default path is written down.
UNIT_EXEC="$(grep -m1 '^ExecStart=' "$SCRIPT_DIR/$SERVICE" | cut -d= -f2-)"
UNIT_EXEC="${UNIT_EXEC//\%h/$HOME}"
THIS_EXEC="$SCRIPT_DIR/preview-poll.sh"
DROPIN_DIR="$UNIT_DIR/$SERVICE.d"

if [[ "$UNIT_EXEC" == "$THIS_EXEC" ]]; then
  rm -rf "$DROPIN_DIR"
else
  echo "this checkout is $SCRIPT_DIR, not the $(dirname "$UNIT_EXEC") the unit names;"
  echo "writing a drop-in so the timer runs this checkout's copy"
  mkdir -p "$DROPIN_DIR"
  # The empty ExecStart= is required: systemd appends to ExecStart= lists, so
  # without the reset both scripts would run, in sequence.
  cat >"$DROPIN_DIR/checkout.conf" <<DROPIN
[Service]
ExecStart=
ExecStart=$THIS_EXEC
DROPIN
  chmod 644 "$DROPIN_DIR/checkout.conf"
fi

systemctl --user daemon-reload
systemctl --user enable --now "$TIMER"

systemctl --user list-timers --all --no-pager "$TIMER"
