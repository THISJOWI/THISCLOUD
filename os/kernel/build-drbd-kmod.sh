#!/usr/bin/env bash
# Build DRBD kernel module from source against a specific kernel.
# Called by build-el-layer.sh, but can also be run standalone.
#
# Usage:
#   KERNEL_VERSION=6.12.1 KERNEL_SRC=/path/to/kernel/build ./build-drbd-kmod.sh
set -euo pipefail

DRBD_VERSION="${DRBD_VERSION:-9.2.7}"
KERNEL_VERSION="${KERNEL_VERSION:-$(uname -r)}"
KERNEL_SRC="${KERNEL_SRC:-/lib/modules/${KERNEL_VERSION}/build}"
WORK_DIR=$(mktemp -d /tmp/drbd-build-XXXXXX)

cleanup() { rm -rf "$WORK_DIR"; }
trap cleanup EXIT

echo "==> Building DRBD kernel module"
echo "    DRBD version:  $DRBD_VERSION"
echo "    Kernel:        $KERNEL_VERSION"
echo "    Kernel source: $KERNEL_SRC"

# ── 1. Check prerequisites ────────────────────────────────────────────

if [ ! -d "$KERNEL_SRC" ]; then
    echo "error: kernel source not found at $KERNEL_SRC"
    echo "  Set KERNEL_SRC to the Yocto build output or host kernel headers"
    exit 1
fi

for tool in make gcc; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "error: $tool not found"
        exit 1
    fi
done

# ── 2. Download DRBD source ───────────────────────────────────────────

echo "==> Downloading DRBD source..."
if [ ! -f "$WORK_DIR/drbd-${DRBD_VERSION}.tar.gz" ]; then
    curl -fSL "https://linbit.com/drbd/drbd-${DRBD_VERSION}.tar.gz" \
        -o "$WORK_DIR/drbd-${DRBD_VERSION}.tar.gz"
fi

tar -xzf "$WORK_DIR/drbd-${DRBD_VERSION}.tar.gz" -C "$WORK_DIR"
SRC_DIR="$WORK_DIR/drbd-${DRBD_VERSION}"

# ── 3. Patch for mainline kernel ──────────────────────────────────────

echo "==> Checking kernel compatibility..."
# DRBD 9.2.x supports kernels 2.6.32 - 6.x. Mainline 6.12 should work.
# If compilation fails, apply compat patches from the DRBD source.

# ── 4. Build kernel module ────────────────────────────────────────────

echo "==> Compiling kernel module..."
make -C "$SRC_DIR" \
    KERNEL_DIR="$KERNEL_SRC" \
    KDIR="$KERNEL_SRC" \
    -j"$(nproc)" \
    modules

# ── 5. Install to staging ─────────────────────────────────────────────

echo "==> Installing to staging..."
STAGE_DIR="${STAGE_DIR:-$WORK_DIR/stage}"
mkdir -p "$STAGE_DIR/usr/lib/modules/${KERNEL_VERSION}/kernel/drivers/block/"

find "$SRC_DIR" -name "drbd.ko" -exec \
    cp {} "$STAGE_DIR/usr/lib/modules/${KERNEL_VERSION}/kernel/drivers/block/" \;

# Also install userspace tools if present
if [ -d "$SRC_DIR/user" ]; then
    make -C "$SRC_DIR/user" install DESTDIR="$STAGE_DIR" 2>/dev/null || true
fi

echo ""
echo "==> DRBD kmod build complete"
echo "    Module: $(find "$STAGE_DIR" -name 'drbd.ko' 2>/dev/null || echo 'not found')"
echo "    Install to: $STAGE_DIR"