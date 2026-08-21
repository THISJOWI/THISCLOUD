#!/usr/bin/env bash
# Build the local RPM repository used by the kickstart. Copies cross-compiled
# RPMs into os/repo/thiscloud and (re)creates repo metadata. Usable on macOS
# for the repo layout, but createrepo_c only runs on Linux — so this must be
# executed on the AlmaLinux 9 builder when createrepo_c is available.
set -euo pipefail

SOURCE="${BASH_SOURCE[0]}"
while [ -L "$SOURCE" ]; do
  DIR="$(cd "$(dirname "$SOURCE")" && pwd)"
  SOURCE="$(readlink "$SOURCE")"
  [[ "$SOURCE" != /* ]] && SOURCE="$DIR/$SOURCE"
done
PLATFORM_DIR="$(cd "$(dirname "$SOURCE")/../.." && pwd)"
cd "$PLATFORM_DIR"

REPO="os/repo"
RPM_DIR="$REPO/thiscloud"
mkdir -p "$REPO" "$RPM_DIR"

# Copy the generated thiscloud RPMs if present.
cp -f target/x86_64-unknown-linux-gnu/generate-rpm/*.rpm "$RPM_DIR/" 2>/dev/null || true

if command -v createrepo_c >/dev/null 2>&1; then
  createrepo_c "$RPM_DIR"
  echo "==> repo metadata written to $RPM_DIR"
else
  echo "warning: createrepo_c not found; repo metadata must be created on the Linux builder" >&2
fi

ls -lh "$RPM_DIR" || true
