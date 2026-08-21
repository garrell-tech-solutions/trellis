#!/usr/bin/env bash
# Moves the Trellis binary to the current green `trunk` without compiling
# anything locally.
#
# .github/workflows/ci.yml's `gate` job already runs
# `cargo build --release --target x86_64-unknown-linux-musl` and asserts it is
# static -- that build IS the release artifact. This script fetches the one
# uploaded from the most recent successful run of that workflow on `trunk`,
# so the binary this host runs is byte-identical to the one CI proved green,
# and no musl release build ever has to compete with a mutation swarm for CPU
# on this box (issue #98).
#
# Used both for routine updates (binary already installed, service already
# running) and, via install.sh, for the very first install (nothing to
# restart yet). Which case this is gets decided at the bottom, from whether
# the systemd --user unit is known to systemd at all.
set -euo pipefail

REPO="garrell-tech-solutions/trellis"
WORKFLOW="ci.yml"
BRANCH="trunk"
ARTIFACT_NAME="trellis-linux-musl"

BIN_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/trellis"
DB_PATH="$DATA_DIR/trellis.db"
BINARY="$BIN_DIR/trellis"
UNIT="trellis.service"

command -v gh >/dev/null || {
  echo "gh (GitHub CLI) is required and not on PATH" >&2
  exit 1
}

RUN_ID="$(gh run list \
  --repo "$REPO" \
  --workflow "$WORKFLOW" \
  --branch "$BRANCH" \
  --status success \
  --limit 1 \
  --json databaseId \
  --jq '.[0].databaseId')"

if [[ -z "$RUN_ID" ]]; then
  echo "no successful $WORKFLOW run found on $BRANCH" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

echo "fetching $ARTIFACT_NAME from $BRANCH run $RUN_ID"
gh run download "$RUN_ID" \
  --repo "$REPO" \
  --name "$ARTIFACT_NAME" \
  --dir "$WORK_DIR"

NEW_BINARY="$WORK_DIR/trellis"
if [[ ! -f "$NEW_BINARY" ]]; then
  echo "downloaded artifact did not contain a 'trellis' binary" >&2
  exit 1
fi
chmod +x "$NEW_BINARY"

# Sanity check before it ever touches the real data directory: refuse a
# binary that isn't the static build the release gate promises
# (features/release_binary.feature).
LDD_OUTPUT="$(ldd "$NEW_BINARY" 2>&1 || true)"
if [[ "$LDD_OUTPUT" != *"not a dynamic executable"* && "$LDD_OUTPUT" != *"statically linked"* ]]; then
  echo "downloaded binary is not statically linked, refusing to install: $LDD_OUTPUT" >&2
  exit 1
fi

mkdir -p "$BIN_DIR" "$DATA_DIR"

# Install to $BIN_DIR, not target/ -- target/ is rebuilt and cleaned by every
# agent worktree that touches this repo, which is exactly the "179 commits
# ago and nobody noticed" failure #98 exists to close.
#
# mv rather than overwriting BINARY in place: a running process keeps its old
# inode mapped, so this is safe to do while trellis.service is up, and the
# subsequent restart picks up the new file cleanly rather than a
# partially-written one.
install -m 755 "$NEW_BINARY" "$WORK_DIR/trellis.installed"
mv -f "$WORK_DIR/trellis.installed" "$BINARY"

echo "installed $BINARY (from $BRANCH run $RUN_ID)"

# Migrations run before the restart so the new binary never opens the
# database in a schema it doesn't expect.
if [[ -f "$DB_PATH" ]]; then
  "$BINARY" migrate --db "$DB_PATH"
fi

if systemctl --user list-unit-files "$UNIT" --no-legend 2>/dev/null | grep -q "$UNIT"; then
  echo "restarting $UNIT"
  systemctl --user restart "$UNIT"
else
  echo "$UNIT is not installed yet; skipping restart (this run is a bootstrap fetch for install.sh)"
fi
