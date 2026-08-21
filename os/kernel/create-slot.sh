#!/usr/bin/env bash
# Create a complete bootable slot from Yocto image + EL layer.
# This is called during the release build to assemble the final slot.
#
# Usage:
#   ./create-slot.sh \
#     --image /path/to/thiscloud-image.ext4 \
#     --el-layer /path/to/el-layer.squashfs \
#     --kernel /path/to/bzImage \
#     --initrd /path/to/initrd.cpio.gz \
#     --output /path/to/slot-output \
#     --version 0.4.0
set -euo pipefail

VERSION="${VERSION:-0.1.0}"
OUTPUT=""
IMAGE=""
EL_LAYER=""
KERNEL=""
INITRD=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --image) IMAGE="$2"; shift 2 ;;
        --el-layer) EL_LAYER="$2"; shift 2 ;;
        --kernel) KERNEL="$2"; shift 2 ;;
        --initrd) INITRD="$2"; shift 2 ;;
        --output) OUTPUT="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

for var in IMAGE KERNEL INITRD OUTPUT; do
    eval "val=\$$var"
    if [ -z "$val" ]; then
        echo "error: --$(echo $var | tr '[:upper:]' '[:lower:]') is required"
        exit 1
    fi
done

echo "==> Creating slot (version $VERSION)"
echo "    Image:    $IMAGE"
echo "    EL layer: ${EL_LAYER:-none}"
echo "    Kernel:   $KERNEL"
echo "    Initrd:   $INITRD"

# ── 1. Create slot directory ──────────────────────────────────────────

mkdir -p "$OUTPUT"
SLOT_DIR="$OUTPUT/slot"
mkdir -p "$SLOT_DIR"

# ── 2. Copy kernel and initrd ─────────────────────────────────────────

echo "==> Copying kernel and initrd..."
cp "$KERNEL" "$SLOT_DIR/vmlinuz"
cp "$INITRD" "$SLOT_DIR/initrd"

# ── 3. Prepare rootfs ─────────────────────────────────────────────────

echo "==> Preparing rootfs..."
ROOTFS_DIR="$SLOT_DIR/rootfs"
mkdir -p "$ROOTFS_DIR"

if [[ "$IMAGE" == *.squashfs ]]; then
    # Already a squashfs — just copy it
    cp "$IMAGE" "$SLOT_DIR/rootfs.squashfs"
elif [[ "$IMAGE" == *.ext4 ]] || [[ "$IMAGE" == *.wic ]]; then
    # Extract ext4 into a directory, then repack as squashfs
    TMP Mount point for loop mount
    TMP_DIR=$(mktemp -d)
    sudo mount -o loop "$IMAGE" "$TMP_DIR" 2>/dev/null || {
        echo "warning: could not mount image, copying raw"
        cp "$IMAGE" "$SLOT_DIR/rootfs.raw"
    }
    if mountpoint -q "$TMP_DIR" 2>/dev/null; then
        cp -a "$TMP_DIR"/. "$ROOTFS_DIR/"
        sudo umount "$TMP_DIR"
    fi
    rmdir "$TMP_DIR" 2>/dev/null || true

    # Pack rootfs as squashfs
    mksquashfs "$ROOTFS_DIR" "$SLOT_DIR/rootfs.squashfs" \
        -comp xz -b 1M -no-xattrs -noappend 2>/dev/null
    rm -rf "$ROOTFS_DIR"
else
    echo "error: unsupported image format: $IMAGE"
    exit 1
fi

# ── 4. Include EL layer ──────────────────────────────────────────────

if [ -n "$EL_LAYER" ] && [ -f "$EL_LAYER" ]; then
    echo "==> Including EL layer..."
    cp "$EL_LAYER" "$SLOT_DIR/el-layer.squashfs"
fi

# ── 5. Write manifest ─────────────────────────────────────────────────

echo "==> Writing manifest..."
ROOTFS_HASH=$(sha256sum "$SLOT_DIR/rootfs.squashfs" | cut -d' ' -f1)
KERNEL_HASH=$(sha256sum "$SLOT_DIR/vmlinuz" | cut -d' ' -f1)
INITRD_HASH=$(sha256sum "$SLOT_DIR/initrd" | cut -d' ' -f1)

cat > "$SLOT_DIR/manifest.json" << EOF
{
  "version": "${VERSION}",
  "image_url": "rootfs.squashfs",
  "image_sha256": "${ROOTFS_HASH}",
  "kernel_url": "vmlinuz",
  "kernel_sha256": "${KERNEL_HASH}",
  "initrd_url": "initrd",
  "initrd_sha256": "${INITRD_HASH}",
  "signature": "",
  "el_layer_version": "1.0.0",
  "sysexts": []
}
EOF

echo "${VERSION}" > "$SLOT_DIR/version"

# ── 6. Pack into single squashfs ─────────────────────────────────────

echo "==> Packing slot squashfs..."
FINAL_SQUASH="$OUTPUT/slot.squashfs"
mksquashfs "$SLOT_DIR" "$FINAL_SQUASH" \
    -comp xz -b 1M -no-xattrs -noappend 2>/dev/null

SLOT_HASH=$(sha256sum "$FINAL_SQUASH" | cut -d' ' -f1)
SLOT_SIZE=$(stat -f%z "$FINAL_SQUASH" 2>/dev/null || stat -c%s "$FINAL_SQUASH" 2>/dev/null || echo 0)

# ── 7. Output manifest ───────────────────────────────────────────────

cat > "$OUTPUT/manifest.json" << EOF
{
  "version": "${VERSION}",
  "image_url": "slot.squashfs",
  "image_sha256": "${SLOT_HASH}",
  "kernel_url": "vmlinuz",
  "kernel_sha256": "${KERNEL_HASH}",
  "initrd_url": "initrd",
  "initrd_sha256": "${INITRD_HASH}",
  "signature": "",
  "el_layer_version": "1.0.0",
  "sysexts": []
}
EOF

echo ""
echo "==> Slot created successfully"
echo "    Slot:   $FINAL_SQUASH"
echo "    Hash:   $SLOT_HASH"
echo "    Size:   $SLOT_SIZE bytes"
echo "    Manifest: $OUTPUT/manifest.json"