#!/usr/bin/env bash
# Cross-compile thiscloudd and thiscloud CLI for the AlmaLinux 9 x86_64 ISO.
# Uses musl target for a fully static binary — no glibc version dependency.
#
#   ./scripts/cross-compile.sh
set -euo pipefail

SOURCE="${BASH_SOURCE[0]}"
while [ -L "$SOURCE" ]; do
  DIR="$(cd "$(dirname "$SOURCE")" && pwd)"
  SOURCE="$(readlink "$SOURCE")"
  [[ "$SOURCE" != /* ]] && SOURCE="$DIR/$SOURCE"
done
PLATFORM_DIR="$(cd "$(dirname "$SOURCE")/../.." && pwd)"
cd "$PLATFORM_DIR"

TARGET="x86_64-unknown-linux-musl"

echo "==> Ensuring target is installed: ${TARGET}"
rustup target add "${TARGET}" 2>/dev/null || true

# musl-gcc is provided by the musl-tools / musl-cross package.
# On macOS: brew install musl-cross → x86_64-linux-musl-gcc
if [ -z "${THISCLOUD_CROSS_LINKER:-}" ]; then
  for c in x86_64-linux-musl-gcc musl-gcc x86_64-linux-gnu-gcc gcc; do
    if command -v "$c" >/dev/null 2>&1; then
      export THISCLOUD_CROSS_LINKER="$c"
      break
    fi
  done
fi
if [ -n "${THISCLOUD_CROSS_LINKER:-}" ]; then
  export CC_x86_64_unknown_linux_musl="${THISCLOUD_CROSS_LINKER}"
  export CXX_x86_64_unknown_linux_musl="${THISCLOUD_CROSS_LINKER}"
fi

echo "==> Building release binaries for ${TARGET}"
cargo build --release --target "${TARGET}"

echo "==> Cross-compile complete"
ls -lh "target/${TARGET}/release/thiscloudd"
ls -lh "target/${TARGET}/release/thiscloud"
