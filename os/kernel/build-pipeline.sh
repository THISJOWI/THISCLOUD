#!/usr/bin/env bash
# Full THISCLOUD OS build pipeline.
# Orchestrates: Yocto image → EL layer → slot assembly → release artifacts.
#
# Must run on AlmaLinux 9 x86_64.
#
# Usage:
#   VERSION=0.4.0 ./build-pipeline.sh [--clean] [--skip-yocto] [--skip-el]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION="${VERSION:-0.1.0}"
BUILD_DIR="${SCRIPT_DIR}/../build"
OUTPUT="${BUILD_DIR}/release"
SKIP_YOCTO=0
SKIP_EL=0
CLEAN=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --clean) CLEAN=1; shift ;;
        --skip-yocto) SKIP_YOCTO=1; shift ;;
        --skip-el) SKIP_EL=1; shift ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

echo "============================================"
echo "  THISCLOUD OS Build Pipeline v${VERSION}"
echo "============================================"
echo ""

mkdir -p "$OUTPUT"

# ── Phase 1: Yocto image ─────────────────────────────────────────────

if [ "$SKIP_YOCTO" -eq 0 ]; then
    echo "==> [1/4] Building Yocto image..."
    YOCTO_ARGS=""
    [ "$CLEAN" -eq 1 ] && YOCTO_ARGS="--clean"
    bash "$SCRIPT_DIR/build-image.sh" $YOCTO_ARGS
else
    echo "==> [1/4] Skipping Yocto build (--skip-yocto)"
fi

# Find the built image
DEPLOY_DIR="$BUILD_DIR/yocto/tmp/deploy/images"
MACHINE="${MACHINE:-qemu-x86-64}"
KERNEL=$(find "$DEPLOY_DIR/$MACHINE" -name "bzImage" | head -1)
INITRD=$(find "$DEPLOY_DIR/$MACHINE" -name "core-image-*.cpio.gz" | head -1)
ROOTFS=$(find "$DEPLOY_DIR/$MACHINE" -name "thiscloud-image-*.ext4" -o -name "thiscloud-image-*.wic" | head -1)

if [ -z "$KERNEL" ] || [ -z "$INITRD" ] || [ -z "$ROOTFS" ]; then
    echo "error: Yocto build artifacts not found in $DEPLOY_DIR/$MACHINE"
    echo "  Expected: bzImage, core-image-*.cpio.gz, thiscloud-image-*.ext4"
    exit 1
fi

echo "    Kernel: $KERNEL"
echo "    Initrd: $INITRD"
echo "    Rootfs: $ROOTFS"
echo ""

# ── Phase 2: EL layer ─────────────────────────────────────────────────

if [ "$SKIP_EL" -eq 0 ]; then
    echo "==> [2/4] Building EL layer..."
    KERNEL_VERSION=$(strings "$KERNEL" | grep -oP 'Linux version \K[0-9.]+' | head -1 || echo "6.12.1")
    KERNEL_VERSION="$KERNEL_VERSION" bash "$SCRIPT_DIR/build-el-layer.sh" \
        --output "$OUTPUT/el-layer"
else
    echo "==> [2/4] Skipping EL layer build (--skip-el)"
fi

EL_LAYER="$OUTPUT/el-layer/el-layer.squashfs"
if [ ! -f "$EL_LAYER" ]; then
    echo "warning: EL layer not found at $EL_LAYER"
    echo "  Slot will be built without EL layer"
    EL_LAYER=""
fi

# ── Phase 3: Assemble slot ───────────────────────────────────────────

echo "==> [3/4] Assembling slot..."
EL_ARG=""
[ -n "$EL_LAYER" ] && EL_ARG="--el-layer $EL_LAYER"

bash "$SCRIPT_DIR/create-slot.sh" \
    --image "$ROOTFS" \
    --kernel "$KERNEL" \
    --initrd "$INITRD" \
    $EL_ARG \
    --output "$OUTPUT/slot" \
    --version "$VERSION"

# ── Phase 4: Build installer ISO ─────────────────────────────────────

echo "==> [4/4] Building installer ISO..."
bash "$SCRIPT_DIR/build-installer.sh" \
    --slot "$OUTPUT/slot/slot.squashfs" \
    --output "$OUTPUT" \
    --version "$VERSION"

# ── Summary ───────────────────────────────────────────────────────────

echo ""
echo "============================================"
echo "  Build complete"
echo "============================================"
echo ""
echo "Artifacts:"
ls -lh "$OUTPUT"/manifest.json 2>/dev/null || true
ls -lh "$OUTPUT"/slot/*.squashfs 2>/dev/null || true
ls -lh "$OUTPUT"/el-layer/*.squashfs 2>/dev/null || true
echo ""
echo "Next steps:"
echo "  1. Test in QEMU:  ./run-vm.sh"
echo "  2. Boot the slot: thpkg os-update --manifest $OUTPUT/manifest.json"