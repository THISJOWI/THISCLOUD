#!/usr/bin/env bash
# Build THISCLOUD installer ISO.
# Uses ISOLINUX for BIOS boot via El Torito.
#
# Usage:
#   ./build-installer.sh \
#     --rootfs /path/to/rootfs \
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
mkdir -p "$MEDIA"/{boot/isolinux,install}

# Copy kernel and initrd
cp "$KERNEL_FILE" "$MEDIA/boot/vmlinuz"
cp "$INITRD_FILE" "$MEDIA/boot/initrd"

# Create rootfs squashfs
echo "==> Creating rootfs squashfs..."
mksquashfs "$ROOTFS_SOURCE" "$MEDIA/install/rootfs.squashfs" -comp xz -b 1M -no-xattrs >/dev/null 2>&1

# Copy installer
cp "$SCRIPT_DIR/installer.sh" "$MEDIA/install/installer.sh"
chmod +x "$MEDIA/install/installer.sh"

# ── 3. Install ISOLINUX bootloader ──────────────────────────────────

echo "==> Installing ISOLINUX..."

ISOLINUX_BIN=""
for path in /usr/lib/ISOLINUX/isolinux.bin /usr/share/syslinux/isolinux.bin \
    /usr/lib/syslinux/bios/isolinux.bin /usr/share/isolinux/isolinux.bin; do
    if [ -f "$path" ]; then
        ISOLINUX_BIN="$path"
        break
    fi
done

if [ -z "$ISOLINUX_BIN" ]; then
    echo "error: isolinux.bin not found"
    echo "    Searched: /usr/lib/ISOLINUX/, /usr/share/syslinux/, /usr/lib/syslinux/bios/, /usr/share/isolinux/"
    echo "    Install: sudo apt-get install isolinux syslinux syslinux-common"
    exit 1
fi

cp "$ISOLINUX_BIN" "$MEDIA/boot/isolinux/"
echo "    isolinux.bin: $ISOLINUX_BIN"

# Copy required C32 modules
for module in ldlinux.c32 libutil.c32 libcom32.c32 menu.c32; do
    src=$(find /usr -name "$module" 2>/dev/null | head -1 || true)
    if [ -n "$src" ]; then
        cp "$src" "$MEDIA/boot/isolinux/"
    fi
done

# ── 4. Create ISOLINUX config ───────────────────────────────────────

cat > "$MEDIA/boot/isolinux/isolinux.cfg" << ISOCFG
DEFAULT thiscloud
TIMEOUT 50
MENU TITLE THISCLOUD OS Installer

LABEL thiscloud
  MENU LABEL THISCLOUD OS Installer
  LINUX /boot/vmlinuz
  APPEND initrd=/boot/initrd console=tty0 quiet

LABEL safe
  MENU LABEL THISCLOUD OS (Safe Mode)
  LINUX /boot/vmlinuz
  APPEND initrd=/boot/initrd console=tty0 nomodeset
ISOCFG

# ── 5. Build ISO with xorriso ───────────────────────────────────────

echo "==> Building ISO..."
ISO_FILE="$OUTPUT/ThisCloud-${VERSION}-installer-x86_64.iso"

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
        -c boot/isolinux/boot.cat \
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
