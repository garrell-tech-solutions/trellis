#!/usr/bin/env bash
# ops/preview.sh <branch>
#
# Puts one branch's CI-built binary on the preview port, without compiling
# anything locally -- the same reasoning ops/update.sh gives for `trunk`
# (issue #98), applied to whatever branch is under review instead (issue
# #102). .github/workflows/ci.yml's `gate` job already runs
# `cargo build --release --target x86_64-unknown-linux-musl` and asserts it
# is static, for every branch with an open pull request, not only `trunk`;
# this script fetches the one uploaded from that branch's most recent
# successful run.
#
# One preview slot, not one per branch: the owner is one person with one
# phone, so one port, one systemd unit and last-writer-wins on `preview.db`
# is the whole design. Running this against a second branch does not queue
# behind the first -- it stops the unit, replaces the binary, wipes the
# database and starts over, exactly like running it twice against the same
# branch (idempotent, per issue #102's "done" criteria). A slot per branch
# would need a port allocator and something to reap abandoned ones for a
# problem this host does not have.
#
# Sequence: download -> verify static -> install to a binary path SEPARATE
# from the live trellis binary -> stop the preview unit -> wipe preview.db so
# every preview starts from that branch's own fresh schema -> migrate ->
# start the preview unit -> wait for it to answer -> reseed through its own
# HTTP surface (ops/preview-seed.sh -- never hand-written SQL, see that
# script's header for why).
#
# What this never touches: trellis.service, ~/.local/bin/trellis, or
# ~/.local/share/trellis/trellis.db -- the live instance, the owner's real
# data. If any of those three would need to change, stop; that is not this
# script's job.
set -euo pipefail

REPO="garrell-tech-solutions/trellis"
WORKFLOW="ci.yml"
ARTIFACT_NAME="trellis-linux-musl"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

BIN_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/trellis"
UNIT_DIR="$HOME/.config/systemd/user"
DB_PATH="$DATA_DIR/preview.db"
BINARY="$BIN_DIR/trellis-preview"
UNIT="trellis-preview.service"
ADDR="127.0.0.1:8081"
TAILSCALE_PORT=8443

BRANCH="${1:?usage: preview.sh <branch>}"

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
  echo "(a run only exists once a pull request is open for this branch -- ci.yml's push trigger fires on trunk only)" >&2
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

# Same sanity check ops/update.sh runs before a downloaded binary ever
# touches a real data directory: refuse one that isn't the static build the
# release gate promises (features/release_binary.feature).
LDD_OUTPUT="$(ldd "$NEW_BINARY" 2>&1 || true)"
if [[ "$LDD_OUTPUT" != *"not a dynamic executable"* && "$LDD_OUTPUT" != *"statically linked"* ]]; then
  echo "downloaded binary is not statically linked, refusing to install: $LDD_OUTPUT" >&2
  exit 1
fi

mkdir -p "$BIN_DIR" "$DATA_DIR" "$UNIT_DIR"

# Installed as trellis-preview, never as trellis -- see this script's header.
# mv rather than overwriting in place, same reason ops/update.sh does it: a
# running preview keeps its old inode mapped, so this is safe even while the
# unit is up, and the stop below picks up the new file cleanly rather than a
# partially-written one.
install -m 755 "$NEW_BINARY" "$WORK_DIR/trellis-preview.installed"
mv -f "$WORK_DIR/trellis-preview.installed" "$BINARY"
echo "installed $BINARY (from $BRANCH run $RUN_ID)"

install -m 644 "$SCRIPT_DIR/trellis-preview.service" "$UNIT_DIR/$UNIT"
systemctl --user daemon-reload

# Stop before wiping the database. T-sqlite-sqlx: preview.db runs in WAL
# mode, so a live writer's -wal/-shm files are part of the database's actual
# state -- deleting the main file out from under a running process is not
# the clean reseed this script means to do.
systemctl --user stop "$UNIT" 2>/dev/null || true
rm -f "$DB_PATH" "$DB_PATH-wal" "$DB_PATH-shm"

# Migrations, on this branch's own binary, against a database that a moment
# ago did not exist. This always applies cleanly: CI's migration-immutability
# job already refuses any PR that edits a migration already on trunk, so
# every branch's migration set is trunk's plus that branch's own append-only
# additions, and sqlx applies the lot to an empty database the same way it
# would to a brand new install. There is no "migrations outrun the seed"
# schema failure to have here -- what can happen instead is the seed's fixed
# HTTP payload no longer matching what a changed triage contract requires,
# which is ops/preview-seed.sh's problem, not this migration step's; see its
# header.
"$BINARY" migrate --db "$DB_PATH"

systemctl --user enable --now "$UNIT"

echo "waiting for $UNIT to answer on $ADDR"
UP=0
for _ in $(seq 1 30); do
  if curl -sf "http://$ADDR/" >/dev/null; then
    UP=1
    break
  fi
  sleep 1
done
if [[ "$UP" -ne 1 ]]; then
  echo "$UNIT did not answer on $ADDR within 30s" >&2
  systemctl --user status --no-pager "$UNIT" >&2 || true
  exit 1
fi

"$SCRIPT_DIR/preview-seed.sh" "http://$ADDR"

# tailscale set --operator=jeremy is already granted on this host, so this
# needs no sudo. `serve`, never `funnel` -- R-multi-tenancy means Trellis has
# no authentication of its own, so the tailnet is the entire security
# boundary; funnel would put the preview on the public internet.
# Idempotent: re-running with the same mapping just confirms it.
tailscale serve --bg --https="$TAILSCALE_PORT" "$ADDR"

TAILNET_HOST="$(tailscale status --json | jq -r '.Self.DNSName' | sed 's/\.$//')"
echo "preview of $BRANCH is live at https://$TAILNET_HOST:$TAILSCALE_PORT"
