#!/usr/bin/env bash
# Install every tool required to build the THISCLOUD AlmaLinux 9 x86_64 ISO.
# Run as root (or with sudo) on the AlmaLinux/RHEL 9 builder.
#
#   sudo ./scripts/install-deps.sh
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "error: run as root, e.g. sudo $0" >&2
  exit 1
fi

echo "==> Refreshing system"
dnf -y update

echo "==> Installing ISO-build + RPM toolchain"
dnf -y install \
  curl \
  cpio \
  createrepo_c \
  lorax \
  xorriso \
  pykickstart \
  yum-utils

# genisoimage lives in EPEL on RHEL-family; useful for mkisofs but optional.
dnf -y install genisoimage 2>/dev/null \
  || echo "warning: genisoimage not found (EPEL only); continuing without it"

echo "==> Installing build toolchain (Rust + musl cross-compile)"
# musl-gcc lives in EPEL on RHEL-family. It is required: the Rust musl target
# compiles C deps (aws-lc-sys etc.) against musl headers, so a glibc-only gcc
# produces broken binaries (undefined __isoc23_sscanf).
dnf -y install gcc gcc-c++ make
dnf -y install musl-gcc 2>/dev/null \
  || echo "warning: musl-gcc not found (EPEL only); cross-compile.sh will fail until it is installed"

# Install Go for building the API server
GO_BIN="/usr/local/go/bin/go"
if [ -x "$GO_BIN" ]; then
  echo "==> Go already installed: $($GO_BIN version)"
else
  echo "==> Installing Go"
  GO_VERSION="1.23.4"
  curl -fL "https://go.dev/dl/go${GO_VERSION}.linux-amd64.tar.gz" | tar -C /usr/local -xzf -
  if [ -x "$GO_BIN" ]; then
    echo "    Go installed: $($GO_BIN version)"
  else
    echo "error: Go installation failed" >&2
    exit 1
  fi
fi
# Ensure Go is on PATH for this session and future builds
export PATH="/usr/local/go/bin:$PATH"
echo 'export PATH="/usr/local/go/bin:$PATH"' > /etc/profile.d/go.sh

# Install Node.js for building the web UI
NODE_BIN=$(command -v node 2>/dev/null || true)
if [ -n "$NODE_BIN" ] && [ -x "$NODE_BIN" ]; then
  echo "==> Node.js already installed: $(node --version)"
else
  echo "==> Installing Node.js"
  curl -fsSL https://rpm.nodesource.com/setup_20.x | bash -
  dnf -y install nodejs
  NODE_BIN=$(command -v node 2>/dev/null || true)
  if [ -n "$NODE_BIN" ] && [ -x "$NODE_BIN" ]; then
    echo "    Node.js installed: $(node --version)"
  else
    echo "error: Node.js installation failed" >&2
    exit 1
  fi
fi
# Ensure npm/npx are on PATH
export PATH="$(dirname "$(command -v node)"):$PATH"

if command -v rustup >/dev/null 2>&1; then
  echo "==> rustup already present, updating"
  rustup update
else
  echo "==> Installing rustup"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi

echo "==> Installing x86_64-unknown-linux-gnu Rust target"
rustup target add x86_64-unknown-linux-gnu

echo "==> Installing cargo-generate-rpm"
cargo install cargo-generate-rpm --locked

# ── Calamares builder deps (compile from source for EL9) ─────────────
echo "==> Installing Calamares/KPMcore build deps"
dnf install -y \
  gcc-c++ gcc make cmake ninja-build \
  qt6-qtbase-devel qt6-qtsvg-devel qt6-qtdeclarative-devel \
  qt6-qtquickcontrols2-devel qt6-qtquicktemplates2-devel \
  boost-devel yaml-cpp-devel parted-devel \
  extra-cmake-modules kf5-kcoreaddons-devel kf5-ki18n-devel kf5-kconfig-devel \
  python3 python3-devel python3-pyqt6 \
  lorax livemedia-utils createrepo_c rpm-build 2>/dev/null \
  || echo "WARNING: some Calamares deps unavailable from current repos (EPEL9 may be needed)"

echo "==> Verifying tools"
MISSING=0
for tool in curl cpio createrepo_c xorriso rustup cargo; do
  if command -v "$tool" >/dev/null 2>&1; then
    echo "  ok: $tool -> $(command -v "$tool")"
  else
    echo "  MISSING: $tool"
    MISSING=1
  fi
done

# Go
if [ -x /usr/local/go/bin/go ]; then
  echo "  ok: go -> /usr/local/go/bin/go ($(/usr/local/go/bin/go version))"
else
  echo "  MISSING: go"
  MISSING=1
fi

# Node.js
if command -v node >/dev/null 2>&1; then
  echo "  ok: node -> $(command -v node) ($(node --version))"
else
  echo "  MISSING: node"
  MISSING=1
fi

if [ "$MISSING" -eq 1 ]; then
  echo ""
  echo "error: some tools are missing. Fix the errors above and re-run."
  exit 1
fi

echo ""
echo "==> All tools verified. You can now build the ISO:"
echo "    sudo -E env PATH=\"$PATH\" \\"
echo "      ALMAISO=/path/to/AlmaLinux-9-latest-x86_64-minimal.iso \\"
echo "      ./scripts/build-iso.sh"