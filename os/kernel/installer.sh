#!/usr/bin/env bash
# THISCLOUD installer — partitions disk, writes slot, installs systemd-boot.
# Runs from the installer ISO. Replaces Calamares.
#
# Usage:
#   ./installer.sh --disk /dev/sda --version 0.4.0
set -euo pipefail

DISK=""
VERSION="${VERSION:-0.1.0}"
SLOT_SRC="/usr/share/installer/slot.squashfs"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --disk) DISK="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

if [ -z "$DISK" ]; then
    echo "error: --disk is required (e.g. --disk /dev/sda)"
    echo "Available disks:"
    lsblk -dno NAME,SIZE,MODEL | grep -v loop
    exit 1
fi

echo "============================================"
echo "  THISCLOUD Installer v${VERSION}"
echo "============================================"
echo "  Target disk: $DISK"
echo ""

# ── 1. Confirm ───────────────────────────────────────────────────────

echo "WARNING: This will ERASE ALL DATA on $DISK"
echo ""
read -p "Type 'YES' to continue: " CONFIRM
if [ "$CONFIRM" != "YES" ]; then
    echo "Aborted."
    exit 1
fi

# ── 2. Partition disk (GPT) ─────────────────────────────────────────

echo ""
echo "==> Partitioning $DISK..."
sync
wipefs -a "$DISK" 2>/dev/null || true
parted -s "$DISK" mklabel gpt

# ESP: 512MiB
parted -s "$DISK" mkpart ESP fat32 1MiB 513MiB
parted -s "$DISK" set 1 esp on

# Slot A: 4GiB
parted -s "$DISK" mkpart "Slot A" ext4 513MiB 4609MiB

# Slot B: 4GiB
parted -s "$DISK" mkpart "Slot B" ext4 4609MiB 8705MiB

# Data: remainder
parted -s "$DISK" mkpart "Data" ext4 8705MiB 100%

partprobe "$DISK"
sleep 2

# Resolve partition names
P1="${DISK}1"
P2="${DISK}2"
P3="${DISK}3"
P4="${DISK}4"

# Handle NVMe (p1/p2/p3/p4 suffix)
if [[ "$DISK" == *nvme* ]] || [[ "$DISK" == *loop* ]]; then
    P1="${DISK}p1"
    P2="${DISK}p2"
    P3="${DISK}p3"
    P4="${DISK}p4"
fi

echo "    Partitions created:"
lsblk -no NAME,SIZE,FSTYPE "$DISK" 2>/dev/null | head -5

# ── 3. Format partitions ─────────────────────────────────────────────

echo ""
echo "==> Formatting partitions..."
mkfs.vfat -F32 -n ESP "$P1"
mkfs.ext4 -L "slot-a" "$P2"
mkfs.ext4 -L "slot-b" "$P3"
mkfs.ext4 -L "data" "$P4"

# ── 4. Install systemd-boot ──────────────────────────────────────────

echo ""
echo "==> Installing systemd-boot..."
ESP_DIR=$(mktemp -d)
mount "$P1" "$ESP_DIR"

bootctl install --esp="$ESP_DIR" 2>/dev/null || {
    echo "    bootctl not found, installing manually..."
    mkdir -p "$ESP_DIR/EFI/systemd"
    mkdir -p "$ESP_DIR/EFI/BOOT"
    mkdir -p "$ESP_DIR/loader/entries"
}

# ── 5. Write slot to Slot A ──────────────────────────────────────────

echo ""
echo "==> Writing slot to Slot A..."
SLOT_DIR=$(mktemp -d)
mount "$P2" "$SLOT_DIR"

if [ -f "$SLOT_SRC" ]; then
    unsquashfs -d "$SLOT_DIR" "$SLOT_SRC" >/dev/null 2>&1 || {
        echo "    unsquashfs not available, copying raw"
        cp "$SLOT_SRC" "$SLOT_DIR/rootfs.squashfs"
    }
else
    echo "error: slot not found at $SLOT_SRC"
    umount "$SLOT_DIR" "$ESP_DIR"
    exit 1
fi

# Copy kernel and initrd to ESP for systemd-boot
if [ -f "$SLOT_DIR/vmlinuz" ]; then
    cp "$SLOT_DIR/vmlinuz" "$ESP_DIR/vmlinuz"
fi
if [ -f "$SLOT_DIR/initrd" ]; then
    cp "$SLOT_DIR/initrd" "$ESP_DIR/initrd"
fi

# Create loader entry
mkdir -p "$ESP_DIR/loader/entries"
cat > "$ESP_DIR/loader/entries/thiscloud-a.conf" << EOF
title   ThisCloud A
version ${VERSION}
linux   /vmlinuz
initrd  /initrd
options root=/dev/disk/by-label/data rw console=ttyS0,115200
EOF

cat > "$ESP_DIR/loader/entries/thiscloud-b.conf" << EOF
title   ThisCloud B
version ${VERSION}
linux   /vmlinuz
initrd  /initrd
options root=/dev/disk/by-label/data rw console=ttyS0,115200
EOF

cat > "$ESP_DIR/loader/loader.conf" << 'LOADER'
default  thiscloud-a.conf
timeout  5
console-mode auto
editor   no
LOADER

# ── 6. Mount data partition and set up /var ───────────────────────────

echo ""
echo "==> Setting up data partition..."
DATA_DIR=$(mktemp -d)
mount "$P4" "$DATA_DIR"
mkdir -p "$DATA_DIR/lib/thpkg/slots/a"
mkdir -p "$DATA_DIR/lib/thpkg/slots/b"
echo "a" > "$DATA_DIR/lib/thpkg/active-slot"

# ── 7. Sync and cleanup ──────────────────────────────────────────────

echo ""
echo "==> Syncing..."
sync

umount "$SLOT_DIR" 2>/dev/null || true
umount "$P4" 2>/dev/null || true
umount "$P1" 2>/dev/null || true
rmdir "$SLOT_DIR" "$ESP_DIR" "$DATA_DIR" 2>/dev/null || true

echo ""
echo "============================================"
echo "  Installation complete"
echo "============================================"
echo ""
echo "  Disk:  $DISK"
echo "  Slot A: installed (version ${VERSION})"
echo "  Slot B: empty (for updates)"
echo "  Data:   configured"
echo ""
echo "  Remove the installer media and reboot."