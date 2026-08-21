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
# On AlmaLinux/RHEL 9: dnf install musl-gcc (EPEL) → musl-gcc
# NOTE: a plain glibc gcc is NOT a valid CC for the musl target — aws-lc-sys
# compiles C against the system headers and links against musl libc, so a
# glibc-only compiler produces broken binaries (e.g. undefined
# __isoc23_sscanf). Fail loudly instead of silently building something broken.
if [ -z "${THISCLOUD_CROSS_LINKER:-}" ]; then
  for c in x86_64-linux-musl-gcc musl-gcc; do
    if command -v "$c" >/dev/null 2>&1; then
      export THISCLOUD_CROSS_LINKER="$c"
      break
    fi
  done
fi
if [ -z "${THISCLOUD_CROSS_LINKER:-}" ]; then
  echo "error: no musl-capable C compiler found (tried: x86_64-linux-musl-gcc musl-gcc)" >&2
  echo "  Install one and re-run, or set THISCLOUD_CROSS_LINKER=<path>" >&2
  echo "  - AlmaLinux/RHEL 9: dnf install musl-gcc (EPEL)" >&2
  echo "  - Debian/Ubuntu:    apt-get install musl-tools" >&2
  echo "  - macOS (brew):     brew install musl-cross" >&2
  exit 1
fi
export CC_x86_64_unknown_linux_musl="${THISCLOUD_CROSS_LINKER}"
export CXX_x86_64_unknown_linux_musl="${THISCLOUD_CROSS_LINKER}"

echo "==> Building release binaries for ${TARGET}"
cargo build --release --target "${TARGET}"

echo "==> Cross-compile complete"
ls -lh "target/${TARGET}/release/thiscloudd"
ls -lh "target/${TARGET}/release/thiscloud"
