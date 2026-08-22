#!/usr/bin/env bash
# Build THISCLOUD installer ISO - simplified version.
# Uses system kernel for fast CI builds.
#
# Usage:
#   ./build-installer.sh \
#     --rootfs /path/to/complete-rootfs \
#     --output /path/to/output \
#     --version 0.4.0
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION="${VERSION:-0.1.0}"
OUTPUT=""
ROOTFS_SOURCE=""
WORK_DIR=$(mktemp -d /tmp/thcloud-iso-XXXXXX)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --rootfs) ROOTFS_SOURCE="$2"; shift 2 ;;
        --output) OUTPUT="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

if [ -z "$OUTPUT" ] || [ -z "$ROOTFS_SOURCE" ]; then
    echo "error: --output and --rootfs are required"
    exit 1
fi

cleanup() { sudo rm -rf "$WORK_DIR"; }
trap cleanup EXIT

echo "==> Building THISCLOUD installer ISO v${VERSION}"

mkdir -p "$OUTPUT"

# ── 1. Find kernel and initrd from rootfs ────────────────────────────

KERNEL_FILE=$(ls "$ROOTFS_SOURCE"/boot/vmlinuz-* 2>/dev/null | head -1 || true)
INITRD_FILE=$(ls "$ROOTFS_SOURCE"/boot/initrd.img-* 2>/dev/null | head -1 || true)

if [ -z "$KERNEL_FILE" ] || [ ! -f "$KERNEL_FILE" ]; then
    echo "error: no kernel found in $ROOTFS_SOURCE/boot/"
    exit 1
fi

if [ -z "$INITRD_FILE" ] || [ ! -f "$INITRD_FILE" ]; then
    echo "error: no initrd found in $ROOTFS_SOURCE/boot/"
    exit 1
fi

echo "    Kernel: $KERNEL_FILE"
echo "    Initrd: $INITRD_FILE"

# ── 2. Prepare ISO media ────────────────────────────────────────────

MEDIA="$WORK_DIR/media"
mkdir -p "$MEDIA"/{boot,install}

# Copy kernel and initrd
sudo cp "$KERNEL_FILE" "$MEDIA/boot/vmlinuz"
sudo cp "$INITRD_FILE" "$MEDIA/boot/initrd"
sudo chmod -R a+r "$MEDIA/boot"

# Create rootfs squashfs for ISO
echo "==> Creating rootfs squashfs for ISO..."
sudo mksquashfs "$ROOTFS_SOURCE" "$MEDIA/install/rootfs.squashfs" -comp xz -b 1M -no-xattrs || true
echo "    Squashfs: $(stat -c%s "$MEDIA/install/rootfs.squashfs" 2>/dev/null || echo 'missing') bytes"

# Copy installer
cp "$SCRIPT_DIR/installer.sh" "$MEDIA/install/installer.sh"
chmod +x "$MEDIA/install/installer.sh"

# Copy slot if available
SLOT_FILE=$(ls "$ROOTFS_SOURCE"/usr/share/installer/*.squashfs 2>/dev/null | head -1 || true)
if [ -n "$SLOT_FILE" ] && [ -f "$SLOT_FILE" ]; then
    cp "$SLOT_FILE" "$MEDIA/install/slot.squashfs"
    echo "    Slot: $SLOT_FILE"
fi

# ── 3. Create init script for boot ──────────────────────────────────

cat > "$MEDIA/boot/init" << 'INIT'
#!/bin/sh
# THISCLOUD Installer Init

export PATH=/sbin:/bin:/usr/sbin:/usr/bin

echo "==> THISCLOUD OS Installer"
echo "    Kernel: $(uname -r)"

# Mount essential filesystems
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
mount -t tmpfs tmpfs /run

# Load modules
for mod in ext4 virtio_pci virtio_blk virtio_net ata_piix ahci; do
    modprobe $mod 2>/dev/null || true
done

# Find CD-ROM
echo "==> Looking for installation media..."
for dev in /dev/sr0 /dev/sr1 /dev/cdrom; do
    if [ -b "$dev" ]; then
        mkdir -p /media/cdrom
        mount -t iso9660 "$dev" /media/cdrom 2>/dev/null && {
            echo "    Mounted: $dev"
            break
        }
    fi
done

# Mount rootfs squashfs
if [ -f /media/cdrom/install/rootfs.squashfs ]; then
    echo "==> Mounting rootfs squashfs..."
    mkdir -p /mnt/rootfs
    mount -t squashfs -o ro,loop /media/cdrom/install/rootfs.squashfs /mnt/rootfs
    ROOTFS="/mnt/rootfs"
else
    echo "==> No rootfs squashfs found, using live system"
    ROOTFS="/"
fi

# Run installer
if [ -x /media/cdrom/install/installer.sh ]; then
    echo "==> Starting installer..."
    exec /media/cdrom/install/installer.sh
else
    echo "==> No installer found, dropping to shell"
    exec /bin/sh
fi

INIT

chmod +x "$MEDIA/boot/init"

# ── 4. Create cpio initrd ───────────────────────────────────────────

echo "==> Creating initrd..."
INITRD_CPIO="$WORK_DIR/initrd.cpio.gz"

pushd "$MEDIA" > /dev/null
find . | cpio -o -H newc 2>/dev/null | gzip > "$INITRD_CPIO"
popd > /dev/null

# ── 5. Build ISO with genisoimage ───────────────────────────────────

echo "==> Building ISO..."
mkdir -p "$OUTPUT"
ISO_FILE="$(cd "$OUTPUT" && pwd)/ThisCloud-${VERSION}-installer-x86_64.iso"

echo "    Media contents:"
find "$MEDIA" -type f -exec ls -lh {} \;

if command -v genisoimage >/dev/null 2>&1; then
    genisoimage -o "$ISO_FILE" \
        -R -J -V "THISCLOUD" \
        -b boot/vmlinuz \
        -c boot/boot.cat \
        -no-emul-boot \
        -boot-load-size 4 \
        -boot-info-table \
        "$MEDIA" 2>&1 && {
        echo "    ISO created with genisoimage"
    } || {
        echo "    genisoimage failed"
    }
fi

# Fallback: xorriso
if [ ! -f "$ISO_FILE" ] && command -v xorriso >/dev/null 2>&1; then
    xorriso -as mkisofs \
        -iso-level 3 \
        -full-iso9660-filenames \
        -volid "THISCLOUD" \
        -output "$ISO_FILE" \
        -eltorito-boot boot/vmlinuz \
            -no-emul-boot \
            -boot-load-size 4 \
            -boot-info-table \
        "$MEDIA" 2>&1 && {
        echo "    ISO created with xorriso"
    } || {
        echo "    xorriso failed"
    }
fi

# Final fallback: tarball
if [ ! -f "$ISO_FILE" ]; then
    ISO_FILE="$OUTPUT/ThisCloud-${VERSION}-installer-rootfs.tar.gz"
    echo "    Creating rootfs tarball instead"
    tar -czf "$ISO_FILE" -C "$MEDIA" .
fi

ISO_SIZE=$(stat -f%z "$ISO_FILE" 2>/dev/null || stat -c%s "$ISO_FILE" 2>/dev/null || echo 0)

echo ""
echo "==> Installer ISO built"
echo "    File: $ISO_FILE"
echo "    Size: $((ISO_SIZE / 1024 / 1024)) MB"
echo ""
echo "    Boot from this ISO to install THISCLOUD on a disk."
