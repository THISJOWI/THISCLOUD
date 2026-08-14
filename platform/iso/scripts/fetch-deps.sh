#!/usr/bin/env bash
# Fetch and stage the THISCLOUD dependency matrix into the local RPM repo so
# the kickstart can install everything from one file:// repo. Run on the
# AlmaLinux 9 x86_64 builder.
#
#   ./scripts/fetch-deps.sh
set -euo pipefail

# Ensure Go and Node.js are on PATH
export PATH="/usr/local/go/bin:$PATH"
if command -v node >/dev/null 2>&1; then
  export PATH="$(dirname "$(command -v node)"):$PATH"
fi

# Resolve script directory robustly (works when called from anywhere)
# Handle both absolute and relative paths
SOURCE="${BASH_SOURCE[0]}"
while [ -L "$SOURCE" ]; do
  DIR="$(cd "$(dirname "$SOURCE")" && pwd)"
  SOURCE="$(readlink "$SOURCE")"
  [[ "$SOURCE" != /* ]] && SOURCE="$DIR/$SOURCE"
done
SCRIPT_DIR="$(cd "$(dirname "$SOURCE")" && pwd)"
ISO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ISO_DIR"  # iso/

REPO="repo"
RPM_DIR="$REPO/thiscloud"
mkdir -p "$REPO" "$RPM_DIR"

# --- cloud-hypervisor: GitHub static binary (no OS package) ---
CH_VERSION="53.0"
CH_URL="https://github.com/cloud-hypervisor/cloud-hypervisor/releases/download/v${CH_VERSION}/cloud-hypervisor-static"
if [ ! -f "$REPO/cloud-hypervisor" ]; then
  echo "==> downloading cloud-hypervisor v${CH_VERSION}"
  curl -fL -o "$REPO/cloud-hypervisor" "$CH_URL"
  chmod 755 "$REPO/cloud-hypervisor"
else
  echo "==> cloud-hypervisor already present, skipping download"
fi

# --- Enable repos that carry the external deps ---
# EPEL + CRB for extra packages, NFV SIG for OVN/OVS, ELRepo for DRBD.
dnf install -y epel-release 2>/dev/null || true
dnf config-manager --enable crb 2>/dev/null || true
dnf config-manager --enable nfv-sig 2>/dev/null || true || echo "nfv-sig not available"
dnf config-manager --enable elrepo 2>/dev/null || true || echo "elrepo not available"

# --- Detect AlmaLinux/RHEL version ---
MAJOR_VERSION=$(rpm -E %{rhel} 2>/dev/null || echo "9")
echo "==> detected distro version: AlmaLinux/RHEL ${MAJOR_VERSION}"

# --- Download RPMs so the install works offline ---
# Core packages (always available in AppStream or EPEL)
CORE_PACKAGES=(
  nginx
  qemu-kvm
)

# Optional packages (may not be available on all versions/repos)
OPTIONAL_PACKAGES=(
  etcd
  openvswitch
  ovn
  ovn-central
  drbd-utils
  linstor
  linstor-client
)

for pkg in "${CORE_PACKAGES[@]}"; do
  echo "==> staging ${pkg}"
  dnf download --destdir "$RPM_DIR" "$pkg" 2>/dev/null \
    || yumdownloader --destdir="$RPM_DIR" "$pkg" 2>/dev/null \
    || echo "warning: could not stage ${pkg}; install manually on the ISO"
done

for pkg in "${OPTIONAL_PACKAGES[@]}"; do
  echo "==> staging ${pkg} (optional)"
  dnf download --destdir "$RPM_DIR" "$pkg" 2>/dev/null \
    || yumdownloader --destdir="$RPM_DIR" "$pkg" 2>/dev/null \
    || echo "info: ${pkg} not available in repos; will be installed from network during kickstart"
done

# --- Build the Go API binary if Go is available ---
echo "==> checking for Go to build thiscloud-api"
if command -v go >/dev/null 2>&1; then
  GO_API_DIR="$(cd "$ISO_DIR/../go-api" 2>/dev/null && pwd)"
  if [ -n "${GO_API_DIR:-}" ] && [ -f "${GO_API_DIR}/go.mod" ]; then
    echo "==> building thiscloud-api from ${GO_API_DIR}"
    pushd "$GO_API_DIR" >/dev/null
    CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build \
      -o "$(pwd)/../../iso/repo/thiscloud-api" \
      ./cmd/api-server
    popd >/dev/null
    echo "==> thiscloud-api built -> $REPO/thiscloud-api"
  else
    echo "warning: go-api source not found; place thiscloud-api binary manually in $REPO/"
  fi
else
  echo "warning: go not found; skipping thiscloud-api build"
  echo "         place the pre-built binary in $REPO/thiscloud-api"
fi

# --- Stage systemd service files ---
echo "==> staging systemd service files"
mkdir -p "$REPO/systemd"
cp -f "$ISO_DIR/systemd/"*.service "$REPO/systemd/" 2>/dev/null \
  || echo "warning: no systemd service files found in $ISO_DIR/systemd/"

# Note: createrepo_c is called by build-iso.sh after this script.
# Do NOT call it here to avoid double work.
echo "==> Dependency RPMs staged in $RPM_DIR"
ls -lh "$RPM_DIR" | head -30 || true
echo "==> Binary artifacts in $REPO:"
ls -lh "$REPO"/*.{rpm,service} 2>/dev/null || true
ls -lh "$REPO/cloud-hypervisor" "$REPO/thiscloud-api" 2>/dev/null || true
