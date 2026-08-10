#!/usr/bin/env bash
# cargo-generate-rpm `before-build` hook: copies the cross-compiled binary into
# the standard target/release location referenced by the package assets.
set -euo pipefail

SOURCE="${BASH_SOURCE[0]}"
while [ -L "$SOURCE" ]; do
  DIR="$(cd "$(dirname "$SOURCE")" && pwd)"
  SOURCE="$(readlink "$SOURCE")"
  [[ "$SOURCE" != /* ]] && SOURCE="$DIR/$SOURCE"
done
cd "$(git rev-parse --show-toplevel 2>/dev/null || echo "$(cd "$(dirname "$SOURCE")/../.." && pwd)")"

PKG="${1:-}"
TARGET="${TARGET:-x86_64-unknown-linux-musl}"

SRC="target/${TARGET}/release/${PKG}"
DEST="target/release/${PKG}"

if [ -f "$SRC" ]; then
  mkdir -p "$(dirname "$DEST")"
  cp -f "$SRC" "$DEST"
  chmod 755 "$DEST"
  echo "prepared ${DEST}"
else
  echo "warning: ${SRC} not found (cross-compile first)" >&2
fi