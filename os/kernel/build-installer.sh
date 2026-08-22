#!/usr/bin/env bash
# Build THISCLOUD installer ISO with ISOLINUX BIOS boot.
# Creates a custom initrd with busybox (no Debian initrd dependency).
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

# 1. Find kernel from rootfs
KERNEL_FILE=$(ls "$ROOTFS_SOURCE"/boot/vmlinuz-* 2>/dev/null | head -1 || true)
if [ -z "$KERNEL_FILE" ] || [ ! -f "$KERNEL_FILE" ]; then
    echo "error: no kernel found in $ROOTFS_SOURCE/boot/"
    exit 1
fi
KERNEL_FILE=$(cd "$(dirname "$KERNEL_FILE")" && pwd)/$(basename "$KERNEL_FILE")
echo "    Kernel: $KERNEL_FILE"

# 2. Create custom initrd with busybox
echo "==> Creating custom initrd..."

INITRD_DIR="$WORK_DIR/initrd"
mkdir -p "$INITRD_DIR"/{bin,sbin,etc,proc,sys,dev,tmp,run,usr/bin,usr/sbin}

BUSYBOX=$(ls "$ROOTFS_SOURCE"/bin/busybox* 2>/dev/null | head -1 || true)
if [ -z "$BUSYBOX" ] || [ ! -f "$BUSYBOX" ]; then
    sudo chroot "$ROOTFS_SOURCE" bash -c "apt-get update -qq && apt-get install -y -qq busybox-static" 2>/dev/null || true
    BUSYBOX=$(ls "$ROOTFS_SOURCE"/bin/busybox* 2>/dev/null | head -1 || true)
fi

if [ -z "$BUSYBOX" ] || [ ! -f "$BUSYBOX" ]; then
    echo "error: cannot find busybox"
    exit 1
fi

echo "    busybox: $BUSYBOX"
sudo cp "$BUSYBOX" "$INITRD_DIR/bin/busybox"
sudo chmod +x "$INITRD_DIR/bin/busybox"

cd "$INITRD_DIR/bin"
for cmd in sh cat mount umount mkdir ls cp mv modprobe switch_root grep sed sleep; do
    sudo ln -sf busybox "$cmd" 2>/dev/null || true
done
cd "$SCRIPT_DIR"

# 3. Create /init for initrd
cat > "$WORK_DIR/init" << 'INIT_SCRIPT'
#!/bin/sh
# THISCLOUD installer init (runs from initramfs)
export PATH=/bin:/sbin:/usr/bin:/usr/sbin

echo ""
echo "============================================"
echo "  THISCLOUD OS Installer"
echo "============================================"

# Mount essential filesystems
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
mount -t tmpfs tmpfs /run

# Load modules
for mod in ext4 virtio_pci virtio_blk virtio_net ata_piix ahci sr_mod isofs; do
    modprobe $mod 2>/dev/null || true
done

# Wait for CD-ROM
echo "Waiting for CD-ROM..."
for i in $(seq 1 10); do
    if [ -b /dev/sr0 ]; then
        echo "  Found /dev/sr0"
        break
    fi
    sleep 1
done

# Mount CD-ROM
mkdir -p /media/cdrom
mount -t iso9660 /dev/sr0 /media/cdrom 2>/dev/null && {
    echo "  Mounted CD-ROM at /media/cdrom"
} || {
    echo "  Trying /dev/cdrom..."
    mount -t iso9660 /dev/cdrom /media/cdrom 2>/dev/null && {
        echo "  Mounted CD-ROM at /media/cdrom"
    } || {
        echo "ERROR: Cannot mount installation media"
        echo "Dropping to shell..."
        exec /bin/sh
    }
}

# Check for installer
if [ -x /media/cdrom/install/installer.sh ]; then
    echo ""
    echo "Starting installer..."
    exec /bin/sh /media/cdrom/install/installer.sh
fi

echo "No installer found on media."
echo "Dropping to shell..."
exec /bin/sh
INIT_SCRIPT

sudo cp "$WORK_DIR/init" "$INITRD_DIR/init"
sudo chmod +x "$INITRD_DIR/init"

# 4. Build initrd
INITRD_FILE=$(cd "$(dirname "$WORK_DIR/initrd.cpio.gz")" && pwd)/$(basename "$WORK_DIR/initrd.cpio.gz")
cd "$INITRD_DIR"
find . | sudo cpio -o -H newc 2>/dev/null | gzip > "$INITRD_FILE"
cd "$SCRIPT_DIR"

echo "    initrd: $(stat -c%s "$INITRD_FILE" 2>/dev/null || stat -f%z "$INITRD_FILE" 2>/dev/null) bytes"

# 5. Prepare ISO media
MEDIA="$WORK_DIR/media"
mkdir -p "$MEDIA"/{boot/isolinux,install}

sudo cp "$KERNEL_FILE" "$MEDIA/boot/vmlinuz"
cp "$INITRD_FILE" "$MEDIA/boot/initrd"

echo "==> Creating rootfs squashfs..."
mksquashfs "$ROOTFS_SOURCE" "$MEDIA/install/rootfs.squashfs" -comp xz -b 1M -no-xattrs >/dev/null 2>&1
echo "    squashfs: $(stat -c%s "$MEDIA/install/rootfs.squashfs" 2>/dev/null || echo 'missing') bytes"

cp "$SCRIPT_DIR/installer.sh" "$MEDIA/install/installer.sh"
chmod +x "$MEDIA/install/installer.sh"

# 6. Install ISOLINUX
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
    exit 1
fi

sudo cp "$ISOLINUX_BIN" "$MEDIA/boot/isolinux/"
echo "    isolinux.bin: $ISOLINUX_BIN"

for module in ldlinux.c32 libutil.c32 libcom32.c32 menu.c32; do
    src=$(find /usr -name "$module" 2>/dev/null | head -1 || true)
    if [ -n "$src" ]; then
        sudo cp "$src" "$MEDIA/boot/isolinux/"
    fi
done

# 7. ISOLINUX config
cat > "$MEDIA/boot/isolinux/isolinux.cfg" << 'ISOCFG'
DEFAULT thiscloud
TIMEOUT 50
MENU TITLE THISCLOUD OS Installer

LABEL thiscloud
  MENU LABEL THISCLOUD OS Installer
  LINUX /boot/vmlinuz
  APPEND initrd=/boot/initrd console=tty0

LABEL safe
  MENU LABEL THISCLOUD OS (Safe Mode)
  LINUX /boot/vmlinuz
  APPEND initrd=/boot/initrd console=tty0 nomodeset
ISOCFG

# 8. Build ISO
echo "==> Building ISO..."
ISO_FILE="$OUTPUT/ThisCloud-${VERSION}-installer-x86_64.iso"

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
    echo "    ISO created"
} || {
    echo "error: xorriso failed"
    exit 1
}

ISO_SIZE=$(stat -c%s "$ISO_FILE" 2>/dev/null || stat -f%z "$ISO_FILE" 2>/dev/null || echo 0)

echo ""
echo "==> Installer ISO built"
echo "    File: $ISO_FILE"
echo "    Size: $((ISO_SIZE / 1024 / 1024)) MB"
