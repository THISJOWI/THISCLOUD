#!/usr/bin/env bash
# Build the THISCLOUD installer ISO.
# Creates a minimal bootable image that runs the installer script.
#
# Usage:
#   ./build-installer.sh \
#     --slot /path/to/slot.squashfs \
#     --output /path/to/output \
#     --version 0.4.0
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION="${VERSION:-0.1.0}"
OUTPUT=""
SLOT=""
WORK_DIR=$(mktemp -d /tmp/thcloud-iso-XXXXXX)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --slot) SLOT="$2"; shift 2 ;;
        --output) OUTPUT="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

if [ -z "$SLOT" ] || [ -z "$OUTPUT" ]; then
    echo "error: --slot and --output are required"
    exit 1
fi

cleanup() { rm -rf "$WORK_DIR"; }
trap cleanup EXIT

echo "==> Building THISCLOUD installer ISO v${VERSION}"

mkdir -p "$OUTPUT" "$WORK_DIR/rootfs"

# ── 1. Create rootfs skeleton ────────────────────────────────────────

ROOTFS="$WORK_DIR/rootfs"
mkdir -p "$ROOTFS"/{bin,sbin,usr/bin,usr/sbin,etc,proc,sys,dev,tmp,run,boot}
mkdir -p "$ROOTFS/usr/lib/systemd/system"
mkdir -p "$ROOTFS/usr/share/installer"
mkdir -p "$ROOTFS/boot/loader/entries"

# Copy installer script
cp "$SCRIPT_DIR/installer.sh" "$ROOTFS/usr/share/installer/installer.sh"
chmod +x "$ROOTFS/usr/share/installer/installer.sh"

# Copy slot squashfs into the media
echo "==> Copying slot into installer media..."
cp "$SLOT" "$ROOTFS/usr/share/installer/slot.squashfs"

# Extract kernel and initrd from the slot
SLOT_TMP=$(mktemp -d)
if command -v unsquashfs >/dev/null 2>&1; then
    unsquashfs -d "$SLOT_TMP" "$SLOT" >/dev/null 2>&1 || true
fi

if [ -f "$SLOT_TMP/vmlinuz" ]; then
    cp "$SLOT_TMP/vmlinuz" "$ROOTFS/boot/vmlinuz"
fi
if [ -f "$SLOT_TMP/initrd" ]; then
    cp "$SLOT_TMP/initrd" "$ROOTFS/boot/initrd"
fi
rm -rf "$SLOT_TMP"

# ── 2. systemd-boot loader config ───────────────────────────────────

cat > "$ROOTFS/boot/loader/loader.conf" << 'LOADER'
default thiscloud-installer.conf
timeout 3
console-mode auto
editor no
LOADER

cat > "$ROOTFS/boot/loader/entries/thiscloud-installer.conf" << EOF
title   THISCLOUD Installer
linux   /vmlinuz
initrd  /initrd
options console=ttyS0,115200 root=/dev/ram0 rdinit=/usr/share/installer/installer.sh
EOF

# ── 3. systemd service to run installer on boot ──────────────────────

cat > "$ROOTFS/usr/lib/systemd/system/thcloud-installer.service" << 'UNIT'
[Unit]
Description=THISCLOUD Installer
After=systemd-modules-load.service

[Service]
Type=oneshot
ExecStart=/usr/share/installer/installer.sh --disk /dev/sda
StandardInput=tty
StandardOutput=journal+console
StandardError=journal+console

[Install]
WantedBy=multi-user.target
UNIT

# ── 4. Install busybox (static) if available ─────────────────────────

if command -v busybox >/dev/null 2>&1; then
    cp "$(which busybox)" "$ROOTFS/bin/busybox"
    for cmd in sh mount umount mkdir cp mv ln chmod sync fdisk sfdisk \
               mkfs.vfat mkfs.ext4 partprobe ls cat echo sleep reboot \
               poweroff halt lsblk findblk blkid; do
        ln -sf busybox "$ROOTFS/bin/$cmd" 2>/dev/null || true
    done
fi

# ── 5. Create initrd (cpio.gz) ──────────────────────────────────────

echo "==> Creating installer initrd..."
INITRD="$WORK_DIR/initrd.cpio.gz"
(cd "$ROOTFS" && find . -print0 | cpio --null -o --format=newc 2>/dev/null | gzip -9) > "$INITRD"

# ── 6. Build ISO with xorriso ───────────────────────────────────────

echo "==> Building ISO..."
ISO_FILE="$OUTPUT/ThisCloud-${VERSION}-installer-x86_64.iso"

# If vmlinuz exists in rootfs, try creating a proper ISO
if [ -f "$ROOTFS/boot/vmlinuz" ]; then
    if command -v xorriso >/dev/null 2>&1; then
        xorriso -as mkisofs \
            -iso-level 3 \
            -full-iso9660-filenames \
            -volid "THISCLOUD" \
            -output "$ISO_FILE" \
            -eltorito-boot "$ROOTFS/boot/vmlinuz" \
                -no-emul-boot \
                -boot-load-size 4096 \
                -boot-info-table \
            "$ROOTFS" 2>/dev/null || true
    fi
fi

# If ISO wasn't created, try genisoimage
if [ ! -f "$ISO_FILE" ] && [ -f "$ROOTFS/boot/vmlinuz" ]; then
    if command -v genisoimage >/dev/null 2>&1; then
        genisoimage -o "$ISO_FILE" \
            -R -J -V "THISCLOUD" \
            -b boot/vmlinuz \
            "$ROOTFS" 2>/dev/null || true
    fi
fi

# If still no ISO, create a tarball of the rootfs
if [ ! -f "$ISO_FILE" ]; then
    ISO_FILE="$OUTPUT/ThisCloud-${VERSION}-installer-rootfs.tar.gz"
    echo "    ISO tools not available or kernel missing, creating rootfs tarball"
    tar -czf "$ISO_FILE" -C "$ROOTFS" .
fi

ISO_SIZE=$(stat -f%z "$ISO_FILE" 2>/dev/null || stat -c%s "$ISO_FILE" 2>/dev/null || echo 0)

echo ""
echo "==> Installer ISO built"
echo "    File: $ISO_FILE"
echo "    Size: $ISO_SIZE bytes"
echo ""
echo "    Boot from this ISO to install THISCLOUD on a disk."