#!/usr/bin/env bash
# Build the THISCLOUD image using Yocto/OpenEmbedded.
# This MUST run on a Linux x86_64 machine (AlmaLinux 9 recommended).
#
# Prerequisites (run install-deps.sh first):
#   - Yocto scarthgap + meta-openembedded layers
#   - git, python3, tar, gzip, bzip2, xz, zstd
#
# Usage:
#   cd os/kernel
#   ./build-image.sh [--clean] [--machine qemu-x86]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
YOCTO_DIR="${SCRIPT_DIR}/build/yocto"
LAYERS_DIR="${SCRIPT_DIR}/meta-thiscloud"
BUILD_DIR="${SCRIPT_DIR}/build"
MACHINE="${MACHINE:-qemu-x86-64}"
DISTRO="poky"
CLEAN=0

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --clean)
            CLEAN=1
            shift
            ;;
        --machine)
            MACHINE="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "==> THISCLOUD image builder (Yocto)"
echo "    Machine: $MACHINE"
echo ""

# ── 1. Initialize Yocto environment ────────────────────────────────────

if [ ! -d "$YOCTO_DIR" ]; then
    echo "==> Cloning Yocto (scarthgap branch)..."
    mkdir -p "$YOCTO_DIR"
    git clone --depth 1 --branch scarthgap \
        https://git.yoctoproject.org/poky "$YOCTO_DIR/poky"
    git clone --depth 1 --branch scarthgap \
        https://git.yoctoproject.org/meta-openembedded "$YOCTO_DIR/meta-openembedded"
fi

if [ ! -f "$YOCTO_DIR/poky/oe-init-build-env" ]; then
    echo "error: Yocto not found at $YOCTO_DIR/poky"
    exit 1
fi

echo "==> Initializing Yocto build environment..."
cd "$YOCTO_DIR/poky"
# shellcheck disable=SC1091
source oe-init-build-env "$BUILD_DIR" > /dev/null 2>&1

# ── 2. Add THISCLOUD layer ────────────────────────────────────────────

echo "==> Adding meta-thiscloud layer..."
bitbake-layers add-layer "$LAYERS_DIR" 2>/dev/null || \
    echo "layer already added or bitbake-layers not available yet"

# Also add meta-openembedded layers
for layer in \
    "$YOCTO_DIR/meta-openembedded/meta-oe" \
    "$YOCTO_DIR/meta-openembedded/meta-python" \
    "$YOCTO_DIR/meta-openembedded/meta-networking"; do
    if [ -d "$layer" ]; then
        bitbake-layers add-layer "$layer" 2>/dev/null || true
    fi
done

# ── 3. Configure local.conf ───────────────────────────────────────────

LOCAL_CONF="$BUILD_DIR/conf/local.conf"
if [ ! -f "$LOCAL_CONF" ]; then
    echo "error: local.conf not found after oe-init-build-env"
    exit 1
fi

echo "==> Configuring build..."
sed -i "s/^MACHINE ?=.*/MACHINE ?= \"${MACHINE}\"/" "$LOCAL_CONF"

# Enable systemd
grep -q 'DISTRO_FEATURES:append.*systemd' "$LOCAL_CONF" || \
    echo 'DISTRO_FEATURES:append = " systemd"' >> "$LOCAL_CONF"

grep -q 'DISTRO_FEATURES:remove.*sysvinit' "$LOCAL_CONF" || \
    echo 'DISTRO_FEATURES:remove = " sysvinit"' >> "$LOCAL_CONF"

# Enable read-only rootfs
grep -q 'IMAGE_FEATURES.*read-only-rootfs' "$LOCAL_CONF" || \
    echo 'IMAGE_FEATURES:append = " read-only-rootfs"' >> "$LOCAL_CONF"

# Set parallelism
NPROC=$(nproc 2>/dev/null || echo 4)
grep -q '^BB_NUMBER_THREADS' "$LOCAL_CONF" || \
    echo "BB_NUMBER_THREADS = \"${NPROC}\"" >> "$LOCAL_CONF"
grep -q '^PARALLEL_MAKE' "$LOCAL_CONF" || \
    echo "PARALLEL_MAKE = \"${NPROC}\"" >> "$LOCAL_CONF"

# ── 4. Build ──────────────────────────────────────────────────────────

if [ "$CLEAN" -eq 1 ]; then
    echo "==> Cleaning build..."
    bitbake thiscloud-image -c cleansstate 2>/dev/null || true
fi

echo "==> Building thiscloud-image..."
bitbake thiscloud-image

echo ""
echo "==> Build complete"
echo "    Image: $BUILD_DIR/tmp/deploy/images/$MACHINE/"
ls -lh "$BUILD_DIR"/tmp/deploy/images/"$MACHINE"/thiscloud-image-* 2>/dev/null || true
echo ""
echo "    Next steps:"
echo "    1. Test in QEMU:  ./run-vm.sh"
echo "    2. Create slot:   ./create-slot.sh <image>"
echo "    3. Build ISO:     ./build-installer.sh"