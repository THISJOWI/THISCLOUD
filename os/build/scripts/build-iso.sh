#!/usr/bin/env bash
# Build a THISCLOUD AlmaLinux 9 ISO. This must run on an AlmaLinux/RHEL 9
# x86_64 builder (xorros/lorax are unavailable on macOS).
# See README.md for the dependency matrix.
#
#   ALMAISO=/path/to/AlmaLinux.iso ./scripts/build-iso.sh
set -euo pipefail

# Ensure Go and Node.js are on PATH
export PATH="/usr/local/go/bin:$PATH"
if command -v node >/dev/null 2>&1; then
  export PATH="$(dirname "$(command -v node)"):$PATH"
fi
# Also source Rust/Cargo env if present
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

# Resolve repo root directory robustly
SOURCE="${BASH_SOURCE[0]}"
while [ -L "$SOURCE" ]; do
  DIR="$(cd "$(dirname "$SOURCE")" && pwd)"
  SOURCE="$(readlink "$SOURCE")"
  [[ "$SOURCE" != /* ]] && SOURCE="$DIR/$SOURCE"
done
REPO_ROOT="$(cd "$(dirname "$SOURCE")/../.." && pwd)"
cd "$REPO_ROOT"

ALMAISO="${ALMAISO:-/data/AlmaLinux-9-latest-x86_64-minimal.iso}"
OUT="${OUT:-/data/thiscloud-iso}"
REPO="os/repo"
RPM_DIR="$REPO/thiscloud"
VERSION="${VERSION:-0.1.0}"

# CI split-build support:
#   THISCLOUD_SKIP_COMPILE=1  skip steps [1-4] — use artifacts pre-staged in
#                             os/repo/ and target/ by another job/runner
#                             (used by the container-based iso-assemble job).
#   THISCLOUD_BUILD_ONLY=1    run steps [1-4] then exit — stage prebuilt
#                             binaries without touching AlmaLinux-only deps
#                             (used by the ubuntu-based iso-build job).
if [ "${THISCLOUD_SKIP_COMPILE:-0}" = "1" ]; then
  echo "==> [1-4/9] skipping compile steps (THISCLOUD_SKIP_COMPILE=1)"
elif [ "${THISCLOUD_BUILD_ONLY:-0}" = "1" ]; then
  echo "==> build-only mode: staging compile artifacts only"
fi

# Check required tools (build-only mode compiles without AlmaLinux-only tools)
if [ "${THISCLOUD_BUILD_ONLY:-0}" != "1" ]; then
  for tool in curl cpio createrepo_c xorriso implantisomd5; do
    if ! command -v "$tool" >/dev/null 2>&1; then
      echo "error: missing required tool: $tool"
      echo "  Install it and re-run, or use: sudo ./os/scripts/install-deps.sh"
      exit 1
    fi
  done

  if ! command -v lorax >/dev/null 2>&1; then
    echo "warning: lorax not found; livemedia-creator ISO build unavailable"
  fi

  # Check ISO exists
  if [ ! -f "$ALMAISO" ]; then
    echo "error: AlmaLinux ISO not found at: $ALMAISO"
    echo "  Set ALMAISO=/path/to/AlmaLinux-9-latest-x86_64-minimal.iso"
    exit 1
  fi
fi

mkdir -p "$REPO" "$RPM_DIR"
# Clean previous build artifacts (OUT only matters when assembling the ISO)
if [ "${THISCLOUD_BUILD_ONLY:-0}" != "1" ]; then
  rm -rf "$OUT"
  mkdir -p "$OUT"
fi

if [ "${THISCLOUD_SKIP_COMPILE:-0}" = "1" ]; then
  echo "==> [1-4/9] skipping compile steps (THISCLOUD_SKIP_COMPILE=1)"
else
echo "==> [1/9] Cross-compile Rust binaries"
bash os/scripts/cross-compile.sh

echo "==> [2/9] Build RPMs with cargo-generate-rpm"
cargo install cargo-generate-rpm --locked 2>/dev/null || true
# RPM versions cannot contain '-' (Cargo prerelease like 0.4.0-alpha is
# invalid). Derive the RPM version from the workspace Cargo.toml and map
# Cargo prereleases to RPM's '~' prerelease separator (0.4.0~alpha).
RPM_VERSION="$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -1)"
# map '-' to RPM's '~' prerelease separator via sed (avoids bash expanding
# '~' to $HOME inside ${var//-/~})
RPM_VERSION="$(printf %s "$RPM_VERSION" | sed 's/-/~/')"
RPM_VER_OPTS=()
if [ -n "$RPM_VERSION" ]; then
  echo "    RPM version: $RPM_VERSION"
  RPM_VER_OPTS=(-s "version = \"$RPM_VERSION\"")
else
  echo "    warning: could not read version from Cargo.toml; using package.version"
fi
# The generate-rpm asset paths in each Cargo.toml point at target/release/, so
# mirror the cross-compiled binaries there (this runs natively on x86_64).
mkdir -p target/release
cp -f target/x86_64-unknown-linux-musl/release/thiscloudd target/release/thiscloudd
cp -f target/x86_64-unknown-linux-musl/release/thiscloud  target/release/thiscloud
cargo generate-rpm --target x86_64-unknown-linux-musl -p thiscloudd "${RPM_VER_OPTS[@]}"
cargo generate-rpm --target x86_64-unknown-linux-musl -p thiscloud-cli "${RPM_VER_OPTS[@]}"
# Copy produced RPMs into the thiscloud sub-repo (kickstart baseurl
# points at /repo/thiscloud on the ISO media).
cp -f target/x86_64-unknown-linux-musl/generate-rpm/*.rpm "$RPM_DIR/" 2>/dev/null || true

# Also copy raw binaries as fallback — kickstart can install them directly
# if RPM install fails (e.g. missing deps or repo issues).
cp -f target/x86_64-unknown-linux-musl/release/thiscloudd "$REPO/thiscloudd" 2>/dev/null || true
cp -f target/x86_64-unknown-linux-musl/release/thiscloud  "$REPO/thiscloud-cli" 2>/dev/null || true
echo "    RPMs in repo: $(ls "$RPM_DIR"/*.rpm 2>/dev/null | wc -l)"
echo "    Raw binaries: thiscloudd=$(test -f "$REPO/thiscloudd" && echo OK || echo MISSING) thiscloud-cli=$(test -f "$REPO/thiscloud-cli" && echo OK || echo MISSING)"

echo "==> [3/9] Build Go API binary"
if command -v go >/dev/null 2>&1; then
  if [ -d go-api ] && [ -f go-api/go.mod ]; then
    pushd go-api >/dev/null
    CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build \
      -o "$OLDPWD/$REPO/thiscloud-api" \
      ./cmd/api-server
    popd >/dev/null
    echo "    go-api built -> $REPO/thiscloud-api"
  else
    echo "    warning: go-api source not found; skipping"
    echo "    Place pre-built thiscloud-api binary in $REPO/"
  fi
else
  echo "    warning: go not found; skipping go-api build"
  echo "    Install Go and re-run, or manually place thiscloud-api in $REPO/"
fi

echo "==> [4/9] Build web-ui (Next.js)"
if command -v node >/dev/null 2>&1; then
  if [ -d web-ui ] && [ -f web-ui/package.json ]; then
    pushd web-ui >/dev/null
    # Install dependencies if needed
    if [ ! -d node_modules ]; then
      echo "    installing web-ui dependencies..."
      npm install 2>/dev/null || pnpm install 2>/dev/null || true
    fi
    # Build the production output
    echo "    building web-ui..."
    npm run build 2>/dev/null || pnpm build 2>/dev/null || true
    # Copy the built output into the ISO repo
    WEBUI_DST="$OLDPWD/$REPO/web-ui"
    mkdir -p "$WEBUI_DST"
    if [ -d .next ]; then
      cp -a .next "$WEBUI_DST/.next"
    fi
    # Copy the server-side standalone output if available (Next.js standalone mode)
    if [ -d .next/standalone ]; then
      cp -a .next/standalone/* "$WEBUI_DST/" 2>/dev/null || true
    fi
    # Copy public assets
    if [ -d public ]; then
      cp -a public "$WEBUI_DST/public" 2>/dev/null || true
    fi
    cp -f package.json "$WEBUI_DST/"
    popd >/dev/null
    echo "    web-ui built -> $REPO/web-ui/"
  else
    echo "    warning: web-ui source not found; skipping"
    echo "    Place pre-built web-ui files in $REPO/web-ui/"
  fi
else
  echo "    warning: node not found; skipping web-ui build"
  echo "    Install Node.js and re-run, or manually place files in $REPO/web-ui/"
fi
fi

if [ "${THISCLOUD_BUILD_ONLY:-0}" = "1" ]; then
  echo ""
  echo "==> Build-only mode (THISCLOUD_BUILD_ONLY=1) — compile artifacts staged in:"
  echo "    repo: $REPO/"
  ls -lh "$REPO" 2>/dev/null | head -20 || true
  echo "    target: target/x86_64-unknown-linux-musl/"
  exit 0
fi

echo "==> [5/9] Stage systemd service files"
mkdir -p "$REPO/systemd"
cp -f os/systemd/*.service "$REPO/systemd/"

# Copy the open-ports script to repo root (kickstart copies it to /usr/local/bin)
cp -f os/scripts/open-ports.sh "$REPO/thiscloud-open-ports"
chmod 755 "$REPO/thiscloud-open-ports"

# Copy the dedicated web-port script
cp -f os/scripts/open-web-port.sh "$REPO/thiscloud-open-web-port"
chmod 755 "$REPO/thiscloud-open-web-port"

# Copy the session-secret generator script
cp -f os/scripts/session-secret.sh "$REPO/thiscloud-session-secret"
chmod 755 "$REPO/thiscloud-session-secret"

echo "==> [6/9] Fetch external dependency RPMs"
bash os/scripts/fetch-deps.sh

echo "==> [7/9] Create local RPM repository metadata"
# createrepo_c must target the sub-directory that contains the RPMs.
# The kickstart repo baseurl is file:///tmp/thiscloud-repo/thiscloud (in %post --nochroot).
if [ -d "$RPM_DIR" ] && [ "$(ls -A "$RPM_DIR"/*.rpm 2>/dev/null)" ]; then
  createrepo_c "$RPM_DIR"
else
  echo "warning: no RPMs found in $RPM_DIR; creating empty repo metadata"
  mkdir -p "$RPM_DIR/repodata"
  createrepo_c "$RPM_DIR" 2>/dev/null || true
fi

echo "==> [8/9] Build live installer ISO (Calamares)"
# The old Anaconda path (make-product-img.sh + remix-iso.sh) is replaced by
# the Calamares live flow. Calamares+KPMcore are compiled from source and
# the live ISO is assembled by livemedia-creator.
ALMAISO="$ALMAISO" OUT="$OUT" VERSION="$VERSION" \
  bash os/calamares/scripts/build-live-iso.sh

echo ""
echo "==> Done"
echo "ISO: $OUT/ThisCloud-${VERSION}-x86_64.iso"
ls -lh "$OUT"/ThisCloud-*.iso 2>/dev/null || echo "  (ISO not created - check build errors above)"
echo ""
echo "Repo contents:"
ls -lhR "$REPO/" 2>/dev/null | head -50 || true
