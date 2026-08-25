#!/usr/bin/env bash
# ops/update.sh [<release-tag>]
#
# Moves the Trellis binary to a GitHub release of this repository, without
# compiling anything locally.
#
#   ops/update.sh                    install the newest release
#   ops/update.sh v2026.08.24.1042   install that one -- THIS IS THE ROLLBACK
#   ops/update.sh --list             show what is installable, newest first
#
# .github/workflows/ci.yml's `gate` job runs
# `cargo build --release --target x86_64-unknown-linux-musl` and asserts the
# result is static -- that build IS the release artifact -- and its `release`
# job attaches that exact file to a GitHub release for every push to `trunk`.
# This script installs one of those, so the binary this host runs is
# byte-identical to the one CI proved green, and no musl release build ever
# has to compete with a mutation swarm for CPU on this box (issue #98).
#
# WHY RELEASES RATHER THAN RUN ARTIFACTS (issue #133). This script used to
# take `--limit 1` of successful trunk pushes and download that run's
# artifact. Two things were wrong with it. Workflow artifacts expire --
# `retention-days: 90` -- so nothing installed was recoverable next quarter
# and there was no other copy. And "the newest green trunk build" was the only
# thing the script could express: there was no way to name a particular build,
# so there was no way to say "the one from before this broke", which is the
# only sentence anyone says at the moment they need it. Releases do not
# expire, they are named, and naming one is the whole of the rollback.
#
# WHAT THIS TRUSTS, AND WHAT IT CHECKS. Four refusals, all of them before
# anything is installed:
#
#   on trunk    the release's tag must resolve to a commit that is an
#               ancestor of THIS repository's `trunk`. This replaces the
#               `--event push` guard the artifact path used (#104) and is
#               strictly stronger: `--event push` established that a push,
#               rather than a fork's pull request, produced the run; this
#               establishes that the specific commit being installed is on
#               the branch the owner reviews and merges to. A release cannot
#               be created here by a fork in the first place -- creating one
#               needs write access to this repository -- so this is the
#               second lock rather than the first.
#   hash        the asset must match the `sha256(trellis) = ...` line the
#               release notes record. That catches a truncated download, a
#               half-written asset, and an asset swapped on an existing
#               release. It is NOT provenance: the hash travels in the same
#               release the asset does, so whoever could replace one could
#               edit the other. Binding the artifact to a reviewed commit
#               properly needs build attestation, which is #109's call to
#               make -- see the `release` job's comment in ci.yml.
#   static      the binary must be the static musl build the release gate
#               promises (features/release_binary.feature).
#   identity    the binary's own `trellis --version` must name the release
#               being installed and the commit it was built from. Without
#               this the other three are checks on a file, not on the thing
#               that ends up running: this script could report that it
#               installed a release and nothing could confirm it.
#
# Used both for routine updates (binary already installed, service already
# running) and, via install.sh, for the very first install (nothing to
# restart yet). Which case this is gets decided at the bottom, from whether
# the systemd --user unit is known to systemd at all.
set -euo pipefail

REPO="garrell-tech-solutions/trellis"
BRANCH="trunk"
ASSET_NAME="trellis"

BIN_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/trellis"
DB_PATH="$DATA_DIR/trellis.db"
BINARY="$BIN_DIR/trellis"
UNIT="trellis.service"

usage() {
  cat <<'USAGE'
usage: ops/update.sh [<release-tag>]

  ops/update.sh                    install the newest release
  ops/update.sh v2026.08.24.1042   install that release by name (rollback)
  ops/update.sh --list             list installable releases, newest first
USAGE
}

for tool in gh jq file sha256sum; do
  command -v "$tool" >/dev/null || {
    echo "$tool is required and not on PATH" >&2
    exit 1
  }
done

REQUESTED_TAG=""
case "${1-}" in
  "") ;;
  -h | --help)
    usage
    exit 0
    ;;
  --list)
    gh release list --repo "$REPO" --exclude-drafts --exclude-pre-releases --limit 30
    exit 0
    ;;
  -*)
    echo "unknown option: $1" >&2
    usage >&2
    exit 2
    ;;
  *) REQUESTED_TAG="$1" ;;
esac

# The newest release, or the one that was asked for. `gh release list`
# excludes drafts and pre-releases explicitly rather than relying on
# `--limit 1` ordering: a draft is a release nobody published and a
# pre-release is one somebody marked as not ready, and installing either
# because it happened to be most recent is the kind of surprise this script
# exists to not have.
if [[ -n "$REQUESTED_TAG" ]]; then
  TAG="$REQUESTED_TAG"
else
  # `|| true` would be wrong here, and was: the first time this was run
  # against a repository with no releases it printed "no published releases"
  # -- correctly, as it happened -- but it printed exactly the same thing
  # when `gh` was merely unauthenticated, because a swallowed error and an
  # empty list are both the empty string. Two different problems with one
  # message is the failure mode T-a-check-must-be-seen-to-fail is about, so
  # the command's own exit status is kept and reported separately from
  # whether it found anything.
  if ! TAG="$(gh release list \
    --repo "$REPO" \
    --exclude-drafts \
    --exclude-pre-releases \
    --limit 1 \
    --json tagName \
    --jq '.[0].tagName' 2>&1)"; then
    echo "FAIL: could not ask $REPO what its releases are." >&2
    echo "      gh said: $TAG" >&2
    exit 1
  fi
  if [[ -z "$TAG" ]]; then
    echo "FAIL: $REPO has no published releases to install." >&2
    echo "      ci.yml's \`release\` job publishes one for every push to $BRANCH;" >&2
    echo "      if that has never run, there is nothing here yet." >&2
    echo "      \`gh release list --repo $REPO\` shows what exists." >&2
    exit 1
  fi
  echo "newest release is $TAG"
fi

if ! RELEASE_JSON="$(gh release view "$TAG" --repo "$REPO" --json tagName,body,assets,isDraft,isPrerelease 2>&1)"; then
  echo "FAIL: no release tagged $TAG in $REPO." >&2
  echo "      gh said: $RELEASE_JSON" >&2
  echo "      \`ops/update.sh --list\` shows what is installable." >&2
  exit 1
fi

if [[ "$(jq -r '.isDraft' <<<"$RELEASE_JSON")" == "true" ]]; then
  echo "FAIL: release $TAG is a draft -- it was never published, and its tag does not exist." >&2
  exit 1
fi

# The commit the tag names. `repos/.../commits/<ref>` resolves a tag the same
# way it resolves a branch or a sha, so this is one call and it fails loudly
# on a tag that exists as a release but not as a ref.
if ! COMMIT="$(gh api "repos/$REPO/commits/$TAG" --jq '.sha' 2>&1)"; then
  echo "FAIL: could not resolve $TAG to a commit in $REPO." >&2
  echo "      gh said: $COMMIT" >&2
  exit 1
fi

# ...and it must be on trunk. `compare/BASE...HEAD` reports `identical` when
# the tag is trunk's tip and `behind` when it is an older commit still
# reachable from trunk. Anything else -- `diverged`, `ahead` -- means the
# commit is not on the branch the owner reviews, and this script does not
# install it however plausible its tag looks.
if ! COMPARE_STATUS="$(gh api "repos/$REPO/compare/$BRANCH...$COMMIT" --jq '.status' 2>&1)"; then
  echo "FAIL: could not compare $COMMIT against $REPO's $BRANCH." >&2
  echo "      gh said: $COMPARE_STATUS" >&2
  exit 1
fi
case "$COMPARE_STATUS" in
  identical | behind) ;;
  *)
    echo "FAIL: release $TAG names commit $COMMIT, which is not on $REPO's $BRANCH (compare says: $COMPARE_STATUS)." >&2
    echo "      Only commits that reached $BRANCH are installed here." >&2
    exit 1
    ;;
esac

echo "release $TAG is commit $COMMIT on $BRANCH"

# The asset, checked for by name before downloading, so a release published
# without its binary says exactly that rather than failing inside gh with a
# message about patterns.
if ! jq -e --arg name "$ASSET_NAME" 'any(.assets[]; .name == $name)' <<<"$RELEASE_JSON" >/dev/null; then
  echo "FAIL: release $TAG carries no asset named '$ASSET_NAME'." >&2
  echo "      It has: $(jq -r '[.assets[].name] | join(", ") | if . == "" then "(no assets at all)" else . end' <<<"$RELEASE_JSON")" >&2
  echo "      A release without its binary is not installable; pick another with \`ops/update.sh --list\`." >&2
  exit 1
fi

# The recorded hash, from the notes the `release` job wrote. Absent means the
# release was made by something other than that job, which is a reason to
# stop rather than a reason to skip the check -- a verification that silently
# turns itself off when its input is missing is the blind spot it was meant
# to close.
# `|| true` because of `set -o pipefail`: `grep` exits 1 on no match, which
# under pipefail fails the whole substitution and, under `set -e`, kills the
# script one line before the message that explains why. Observed doing exactly
# that against a release with no hash line -- exit 1 and not a word about it,
# which is the silent failure this check exists to prevent, reproduced in the
# check itself.
EXPECTED_SHA256="$(jq -r '.body' <<<"$RELEASE_JSON" \
  | grep -oE 'sha256\(trellis\) = [0-9a-f]{64}' \
  | head -1 \
  | sed -E 's/.* = //' || true)"
if [[ -z "$EXPECTED_SHA256" ]]; then
  echo "FAIL: release $TAG's notes record no 'sha256(trellis) = ...' line." >&2
  echo "      ci.yml's \`release\` job always writes one, so this release was not" >&2
  echo "      produced by it, and there is nothing to check the download against." >&2
  exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

echo "fetching $ASSET_NAME from release $TAG"
if ! DOWNLOAD_OUTPUT="$(gh release download "$TAG" \
  --repo "$REPO" \
  --pattern "$ASSET_NAME" \
  --dir "$WORK_DIR" 2>&1)"; then
  echo "FAIL: could not download '$ASSET_NAME' from release $TAG." >&2
  echo "      gh said: $DOWNLOAD_OUTPUT" >&2
  exit 1
fi

NEW_BINARY="$WORK_DIR/$ASSET_NAME"
if [[ ! -f "$NEW_BINARY" ]]; then
  echo "FAIL: release $TAG's download produced no '$ASSET_NAME' file." >&2
  exit 1
fi

ACTUAL_SHA256="$(sha256sum "$NEW_BINARY" | cut -d' ' -f1)"
if [[ "$ACTUAL_SHA256" != "$EXPECTED_SHA256" ]]; then
  echo "FAIL: the downloaded binary is not what release $TAG says it is." >&2
  echo "      release notes: $EXPECTED_SHA256" >&2
  echo "      downloaded:    $ACTUAL_SHA256" >&2
  echo "      Refusing to install it." >&2
  exit 1
fi
echo "sha256 matches the release notes ($ACTUAL_SHA256)"

chmod +x "$NEW_BINARY"

# Sanity check before it ever touches the real data directory: refuse a
# binary that isn't the static build the release gate promises
# (features/release_binary.feature).
#
# `file`, not `ldd`. `ldd` answers "is this dynamically linked" by actually
# running the binary under the dynamic loader -- and for a binary the loader
# considers static, glibc's `ldd` execs it directly and inspects how it
# behaves, rather than merely inspecting it. Either way, the artifact runs
# before this check has decided whether to trust it, and a crafted ELF that
# names its own interpreter would execute at exactly that moment. `file`
# reads the ELF header and answers the same question without ever loading
# or executing what it is looking at.
FILE_OUTPUT="$(file "$NEW_BINARY")"
if [[ "$FILE_OUTPUT" != *"static"* ]]; then
  echo "FAIL: the binary in release $TAG is not statically linked, refusing to install it." >&2
  echo "      file says: $FILE_OUTPUT" >&2
  exit 1
fi

# Only now, with the bytes verified against the release and the ELF header
# read without executing anything, does the binary get run -- and the first
# thing asked of it is who it thinks it is. A mismatch here means the release
# and its asset disagree, and the honest response is to install neither.
EXPECTED_VERSION="trellis $TAG ($COMMIT)"
if ! ACTUAL_VERSION="$("$NEW_BINARY" --version 2>&1)"; then
  echo "FAIL: the binary in release $TAG could not report its version." >&2
  echo "      it said: $ACTUAL_VERSION" >&2
  exit 1
fi
if [[ "$ACTUAL_VERSION" != "$EXPECTED_VERSION" ]]; then
  echo "FAIL: the binary in release $TAG does not identify as that release." >&2
  echo "      release says:  $EXPECTED_VERSION" >&2
  echo "      binary says:   $ACTUAL_VERSION" >&2
  echo "      Refusing to install a binary that misnames itself." >&2
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

echo "installed $BINARY (release $TAG, commit $COMMIT)"

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

# The last word belongs to the file that is now on disk, not to this script's
# own account of what it did. Read back from $BINARY -- so this reports what
# `trellis --version` will say to whoever asks it next, rather than what the
# thing in the temporary directory said a moment ago.
INSTALLED_VERSION="$("$BINARY" --version)"
if [[ "$INSTALLED_VERSION" != "$EXPECTED_VERSION" ]]; then
  echo "FAIL: $BINARY reports \"$INSTALLED_VERSION\" after install, expected \"$EXPECTED_VERSION\"." >&2
  exit 1
fi
echo "$INSTALLED_VERSION"

# Rolling back is naming a release, not reconstructing one.
echo "to roll back:  ops/update.sh <tag>   (\`ops/update.sh --list\` shows the tags)"
