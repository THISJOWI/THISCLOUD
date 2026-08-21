#!/usr/bin/env bash
# Create a complete THISCLOUD OS rootfs using debootstrap.
# Produces a bootable rootfs with systemd, thpkg, first-run, and
# basic system components.
#
# Usage:
#   ./build-rootfs.sh --output /path/to/rootfs --version 0.4.0 \
#       [--kernel /path/to/vmlinuz] [--initrd /path/to/initrd.img]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION="${VERSION:-0.1.0}"
OUTPUT=""
KERNEL=""
INITRD=""
SUITE="bookworm"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --output) OUTPUT="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        --kernel) KERNEL="$2"; shift 2 ;;
        --initrd) INITRD="$2"; shift 2 ;;
        --suite) SUITE="$2"; shift 2 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

if [ -z "$OUTPUT" ]; then
    echo "error: --output is required"
    exit 1
fi

echo "==> Creating THISCLOUD OS rootfs v${VERSION} (suite: ${SUITE})"

# ── 1. Bootstrap base system ────────────────────────────────────────

echo "==> Running debootstrap..."
mkdir -p "$OUTPUT"
sudo debootstrap \
    --variant=minbase \
    --include=systemd,systemd-sysv,dbus,iproute2,iptables,iputils-ping,curl,kmod,udev,initramfs-tools \
    "$SUITE" "$OUTPUT" http://deb.debian.org/debian

# ── 2. Install kernel (from Debian repos — vanilla Linux) ────────────

echo "==> Installing kernel..."
sudo chroot "$OUTPUT" bash -c "apt-get update && apt-get install -y --no-install-recommends linux-image-amd64 systemd-sysv" 2>/dev/null || true

# Find the installed kernel
INSTALLED_VMLINUZ=$(sudo find "$OUTPUT/boot" -name "vmlinuz-*" | head -1)
INSTALLED_INITRD=$(sudo find "$OUTPUT/boot" -name "initrd.img-*" | head -1)

if [ -n "$INSTALLED_VMLINUZ" ]; then
    echo "    Installed kernel: $(basename "$INSTALLED_VMLINUZ")"
fi

# ── 3. Set up THISCLOUD components ─────────────────────────────────

echo "==> Installing THISCLOUD components..."

# Create directory structure
sudo mkdir -p "$OUTPUT/etc/thpkg"
sudo mkdir -p "$OUTPUT/usr/lib/systemd/system"
sudo mkdir -p "$OUTPUT/usr/bin"
sudo mkdir -p "$OUTPUT/usr/share/first-run"
sudo mkdir -p "$OUTPUT/var/lib/extensions"
sudo mkdir -p "$OUTPUT/var/lib/thpkg/slots"
sudo mkdir -p "$OUTPUT/etc/systemd/system/multi-user.target.wants"

# System info
echo "$VERSION" | sudo tee "$OUTPUT/etc/thpkg/version" > /dev/null
echo "thiscloud-${VERSION}" | sudo tee "$OUTPUT/etc/hostname" > /dev/null

# Install thpkg binary (from CI build)
if [ -f "${SCRIPT_DIR}/../../target/release/thpkg" ]; then
    sudo cp "${SCRIPT_DIR}/../../target/release/thpkg" "$OUTPUT/usr/bin/thpkg"
    sudo chmod +x "$OUTPUT/usr/bin/thpkg"
    echo "    Installed thpkg binary"
fi

# Install first-run agent
if [ -f "${SCRIPT_DIR}/../system/first-run/first-run.sh" ]; then
    sudo cp "${SCRIPT_DIR}/../system/first-run/first-run.sh" "$OUTPUT/usr/share/first-run/first-run.sh"
    sudo chmod +x "$OUTPUT/usr/share/first-run/first-run.sh"
fi

# ── 4. systemd services ────────────────────────────────────────────

echo "==> Configuring systemd services..."

# thpkg booted-ok service
if [ -f "${SCRIPT_DIR}/../packages/thpkg/thpkg-booted-ok.service" ]; then
    sudo cp "${SCRIPT_DIR}/../packages/thpkg/thpkg-booted-ok.service" \
        "$OUTPUT/usr/lib/systemd/system/"
    sudo ln -sf /usr/lib/systemd/system/thpkg-booted-ok.service \
        "$OUTPUT/etc/systemd/system/multi-user.target.wants/"
fi

# first-run service
cat > "$OUTPUT/usr/lib/systemd/system/thiscloud-first-run.service" << 'UNIT'
[Unit]
Description=THISCLOUD First Run Configuration
After=network-online.target
Wants=network-online.target
ConditionPathExists=!/etc/thpkg/.first-run-done

[Service]
Type=oneshot
ExecStart=/usr/share/first-run/first-run.sh
ExecStart=/bin/touch /etc/thpkg/.first-run-done
RemainAfterExit=yes
StandardOutput=journal+console
StandardError=journal+console

[Install]
WantedBy=multi-user.target
UNIT

sudo ln -sf /usr/lib/systemd/system/thiscloud-first-run.service \
    "$OUTPUT/etc/systemd/system/multi-user.target.wants/"

# ── 5. Network configuration ────────────────────────────────────────

echo "==> Configuring network..."

mkdir -p "$OUTPUT/etc/systemd/network"
cat > "$OUTPUT/etc/systemd/network/20-wired.network" << 'NETWORK'
[Match]
Name=en* eth*

[Network]
DHCP=yes
NETWORK

# Enable systemd-networkd
sudo ln -sf /usr/lib/systemd/system/systemd-networkd.service \
    "$OUTPUT/etc/systemd/system/multi-user.target.wants/" 2>/dev/null || true
sudo ln -sf /usr/lib/systemd/system/systemd-resolved.service \
    "$OUTPUT/etc/systemd/system/multi-user.target.wants/" 2>/dev/null || true

# ── 6. Bootloader config (systemd-boot) ────────────────────────────

echo "==> Creating bootloader config..."
sudo mkdir -p "$OUTPUT/boot/loader/entries"

sudo tee "$OUTPUT/boot/loader/loader.conf" > /dev/null << 'LOADER'
default thiscloud.conf
timeout 5
console-mode auto
editor no
LOADER

# Determine kernel filename for bootloader entry
KVER=""
for f in "$OUTPUT/boot"/vmlinuz-*; do
    if [ -f "$f" ]; then
        KVER=$(basename "$f" | sed 's/^vmlinuz-//')
        break
    fi
done

if [ -n "$KVER" ]; then
    sudo tee "$OUTPUT/boot/loader/entries/thiscloud.conf" > /dev/null << EOF
title   THISCLOUD OS ${VERSION}
linux   /vmlinuz-${KVER}
initrd  /initrd.img-${KVER}
options root=/dev/sda3 rw console=ttyS0,115200
EOF
fi

# ── 7. Clean up ─────────────────────────────────────────────────────

echo "==> Cleaning up..."
sudo chroot "$OUTPUT" bash -c "apt-get clean && rm -rf /var/lib/apt/lists/*" 2>/dev/null || true
sudo rm -rf "$OUTPUT/tmp/*" "$OUTPUT/var/tmp/*" 2>/dev/null || true

# Report
echo ""
echo "==> Rootfs created"
echo "    Path: $OUTPUT"
echo "    Size: $(du -sh "$OUTPUT" | awk '{print $1}')"
echo "    Kernel: ${KVER:-not found}"
if [ -f "$OUTPUT/boot/vmlinuz-${KVER}" ]; then
    echo "    vmlinuz: $(sudo stat -c%s "$OUTPUT/boot/vmlinuz-${KVER}") bytes"
fi
if [ -f "$OUTPUT/boot/initrd.img-${KVER}" ]; then
    echo "    initrd: $(sudo stat -c%s "$OUTPUT/boot/initrd.img-${KVER}") bytes"
fi
