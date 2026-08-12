#!/usr/bin/env bash
# Executable QA procedure: qa/release_binary.md (covers
# features/release_binary.feature).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

TARGET="x86_64-unknown-linux-musl"
RELEASE_DIR="target/$TARGET/release"

cargo build --release --target "$TARGET"

FAILURES=0

mapfile -t BINARIES < <(find "$RELEASE_DIR" -maxdepth 1 -type f -executable)
if [[ "${#BINARIES[@]}" -ne 1 ]]; then
  echo "FAIL: expected exactly one executable binary under $RELEASE_DIR, found ${#BINARIES[@]}: ${BINARIES[*]}" >&2
  FAILURES=1
else
  BINARY="${BINARIES[0]}"
  LDD_OUTPUT="$(ldd "$BINARY" 2>&1 || true)"
  # ldd's exact wording for "this is not a dynamic executable" differs by
  # implementation/libc: musl and some glibc builds print "not a dynamic
  # executable"; this Ubuntu glibc prints "statically linked" instead. Both
  # mean the same observable outcome the procedure asks for.
  if [[ "$LDD_OUTPUT" != *"not a dynamic executable"* && "$LDD_OUTPUT" != *"statically linked"* ]]; then
    echo "FAIL: ldd on $BINARY did not report the binary as static: $LDD_OUTPUT" >&2
    FAILURES=1
  fi
fi

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: release_binary"
