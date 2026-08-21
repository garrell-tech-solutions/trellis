#!/usr/bin/env bash
# One-time bootstrap: install the Trellis binary, create its data directory,
# and install and enable the systemd --user unit. issue #98.
#
# Idempotent by design (mkdir -p, `enable-linger` and `daemon-reload` are all
# safe to re-run), so re-running this after a fresh checkout does not double
# up anything -- it is meant to be run once per host, not once ever.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

BIN_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/trellis"
UNIT_DIR="$HOME/.config/systemd/user"
UNIT="trellis.service"

mkdir -p "$BIN_DIR" "$DATA_DIR" "$UNIT_DIR"

# The database itself is not created here. #98 moves the owner's existing
# database into $DATA_DIR/trellis.db by hand, via a `sqlite3 ... ".backup"`
# copy (never `cp` -- T-sqlite-sqlx, the database is WAL mode) with the
# original left in place until the new location is confirmed working. A
# fresh install with no existing database gets one created by the first
# `trellis migrate` invocation below, same as any other host.

# Fetch and place the binary. update.sh owns the "how" (gh run download from
# the latest green trunk run, verify it's static, install to $BIN_DIR, run
# migrations) so that logic exists in exactly one place; at this point the
# unit is not installed yet, so update.sh's restart step is a no-op.
"$SCRIPT_DIR/update.sh"

# Linger=no is this host's default: a systemd --user unit dies the moment the
# desktop session ends, silently. enable-linger keeps the user instance (and
# this unit) running from boot with nobody logged in. Safe to re-run.
loginctl enable-linger "$USER"

install -m 644 "$SCRIPT_DIR/trellis.service" "$UNIT_DIR/$UNIT"

systemctl --user daemon-reload
systemctl --user enable --now "$UNIT"
systemctl --user status --no-pager "$UNIT"
