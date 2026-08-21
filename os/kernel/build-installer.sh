#!/usr/bin/env bash
# Build THISCLOUD installer ISO with vanilla kernel.
# Creates a minimal bootable ISO that runs the installer directly.
#
# Usage:
#   ./build-installer.sh \
#     --kernel /path/to/vmlinuz \
#     --initrd /path/to/initrd.img \
#     --rootfs /path/to/complete-rootfs \
#     --output /path/to/output \
#     --version 0.4.0
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION="${VERSION:-0.1.0}"
OUTPUT=""
ROOTFS_SOURCE=""
KERNEL=""
INITRD=""
WORK_DIR=$(mktemp -d /tmp/thcloud-iso-XXXXXX)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --kernel) KERNEL="$2"; shift 2 ;;
        --initrd) INITRD="$2"; shift 2 ;;
        --rootfs) ROOTFS_SOURCE="$2"; shift 2 ;;
        --output) OUTPUT="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

if [ -z "$OUTPUT" ]; then
    echo "error: --output is required"
    exit 1
fi

cleanup() { rm -rf "$WORK_DIR"; }
trap cleanup EXIT

echo "==> Building THISCLOUD installer ISO v${VERSION}"

mkdir -p "$OUTPUT"

# ── 1. Prepare media root ────────────────────────────────────────────

MEDIA="$WORK_DIR/media"
mkdir -p "$MEDIA"/{boot/isolinux,boot/grub,install}

# ── 2. Copy kernel and initrd ─────────────────────────────────────────

echo "==> Installing kernel and initrd..."

# Find kernel
KERNEL_FILE=""
if [ -n "$KERNEL" ] && [ -f "$KERNEL" ]; then
    KERNEL_FILE="$KERNEL"
elif [ -n "$ROOTFS_SOURCE" ]; then
    KERNEL_FILE=$(ls "$ROOTFS_SOURCE"/boot/vmlinuz-* 2>/dev/null | head -1)
fi

if [ -z "$KERNEL_FILE" ] || [ ! -f "$KERNEL_FILE" ]; then
    echo "error: no kernel found"
    exit 1
fi

sudo cp "$KERNEL_FILE" "$MEDIA/boot/vmlinuz"
echo "    Kernel: $KERNEL_FILE"

# Find initrd
INITRD_FILE=""
if [ -n "$INITRD" ] && [ -f "$INITRD" ]; then
    INITRD_FILE="$INITRD"
elif [ -n "$ROOTFS_SOURCE" ]; then
    INITRD_FILE=$(ls "$ROOTFS_SOURCE"/boot/initrd.img-* 2>/dev/null | head -1)
fi

if [ -z "$INITRD_FILE" ] || [ ! -f "$INITRD_FILE" ]; then
    echo "error: no initrd found"
    exit 1
fi

sudo cp "$INITRD_FILE" "$MEDIA/boot/initrd"
echo "    Initrd: $INITRD_FILE"

sudo chmod -R a+r "$MEDIA/boot"

# ── 3. Copy installer script ─────────────────────────────────────────

echo "==> Installing installer..."

# Use the installer script from the OS directory
INSTALLER_SRC="${SCRIPT_DIR}/installer.sh"
if [ -f "$INSTALLER_SRC" ]; then
    echo "    Source: $INSTALLER_SRC"
    cp "$INSTALLER_SRC" "$MEDIA/install/installer.sh"
    chmod +x "$MEDIA/install/installer.sh"
    echo "    Installed installer.sh"
else
    echo "    error: installer.sh not found at $INSTALLER_SRC"
    echo "    SCRIPT_DIR=$SCRIPT_DIR"
    ls -la "$SCRIPT_DIR/" | head -20
    exit 1
fi

# Copy slot squashfs if available
if [ -n "$ROOTFS_SOURCE" ]; then
    SLOT_FILE=$(ls "$ROOTFS_SOURCE"/usr/share/installer/*.squashfs 2>/dev/null | head -1)
    if [ -n "$SLOT_FILE" ] && [ -f "$SLOT_FILE" ]; then
        cp "$SLOT_FILE" "$MEDIA/install/slot.squashfs"
        echo "    Slot: $SLOT_FILE"
    fi
fi

# ── 4. Create boot configuration ─────────────────────────────────────

echo "==> Creating boot configuration..."

# ISOLINUX for BIOS boot
cat > "$MEDIA/boot/isolinux/isolinux.cfg" << 'ISOLINUX'
DEFAULT thiscloud
TIMEOUT 50
MENU TITLE THISCLOUD OS Installer

LABEL thiscloud
  MENU LABEL THISCLOUD OS Installer
  LINUX /boot/vmlinuz
  INITRD /boot/initrd
  APPEND console=tty0

LABEL thiscloud-safe
  MENU LABEL THISCLOUD OS (Safe Mode)
  LINUX /boot/vmlinuz
  INITRD /boot/initrd
  APPEND console=tty0 nomodeset

ISOLINUX

# GRUB config for UEFI boot
mkdir -p "$MEDIA/boot/grub"
cat > "$MEDIA/boot/grub/grub.cfg" << 'GRUB'
set default=0
set timeout=5

menuentry "THISCLOUD OS Installer" {
  linux /boot/vmlinuz console=tty0
  initrd /boot/initrd
}

menuentry "THISCLOUD OS (Safe Mode)" {
  linux /boot/vmlinuz console=tty0 nomodeset
  initrd /boot/initrd
}

GRUB

# ── 5. Build ISO ─────────────────────────────────────────────────────

echo "==> Building ISO..."
ISO_FILE="$OUTPUT/ThisCloud-${VERSION}-installer-x86_64.iso"

# Build ISO with xorriso
if command -v xorriso >/dev/null 2>&1; then
    xorriso -as mkisofs \
        -iso-level 3 \
        -full-iso9660-filenames \
        -volid "THISCLOUD" \
        -output "$ISO_FILE" \
        -eltorito-boot boot/isolinux/isolinux.bin \
            -no-emul-boot \
            -boot-load-size 4 \
            -boot-info-table \
        "$MEDIA" 2>&1 && {
        echo "    ISO created with xorriso"
    } || {
        echo "    xorriso failed"
    }
fi

# Fallback: genisoimage
if [ ! -f "$ISO_FILE" ] && command -v genisoimage >/dev/null 2>&1; then
    genisoimage -o "$ISO_FILE" \
        -R -J -V "THISCLOUD" \
        -b boot/isolinux/isolinux.bin \
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
