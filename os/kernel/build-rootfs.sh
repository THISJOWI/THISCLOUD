#!/usr/bin/env bash
# Create THISCLOUD OS rootfs using debootstrap.
# Uses system kernel for fast CI builds.
#
# Usage:
#   ./build-rootfs.sh --output /path/to/rootfs --version 0.4.0
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION="${VERSION:-0.1.0}"
OUTPUT=""
SUITE="bookworm"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --output) OUTPUT="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        --suite) SUITE="$2"; shift 2 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

if [ -z "$OUTPUT" ]; then
    echo "error: --output is required"
    exit 1
fi

echo "==> Creating THISCLOUD OS rootfs v${VERSION} (suite: ${SUITE})"

echo "==> Running debootstrap..."
mkdir -p "$OUTPUT"
sudo debootstrap \
    --variant=minbase \
    --include=systemd,systemd-sysv,dbus,iproute2,iptables,iputils-ping,curl,kmod,udev,initramfs-tools \
    --include=sudo,vim,nano,less,psmisc,procps,ethtool \
    --include=net-tools,openssh-server,openssh-client,curl,wget \
    --include=e2fsprogs,dosfstools,parted,gdisk,blkid \
    "$SUITE" "$OUTPUT" http://deb.debian.org/debian

echo "==> Installing kernel..."
sudo chroot "$OUTPUT" bash -c "apt-get update && apt-get install -y --no-install-recommends linux-image-amd64 systemd-sysv" 2>/dev/null || true

KVER=$(ls "$OUTPUT"/boot/vmlinuz-* 2>/dev/null | head -1 | xargs basename 2>/dev/null | sed 's/^vmlinuz-//' || true)

if [ -z "$KVER" ]; then
    echo "error: no kernel installed"
    exit 1
fi

echo "    Installed kernel: $KVER"

echo "==> Generating initrd for ${KVER}..."
sudo mount --bind /dev "$OUTPUT/dev" 2>/dev/null || true
sudo mount -t proc proc "$OUTPUT/proc" 2>/dev/null || true
sudo mount -t sysfs sysfs "$OUTPUT/sys" 2>/dev/null || true

sudo chroot "$OUTPUT" bash -c "mkinitramfs -o /boot/initrd.img-${KVER} ${KVER}" 2>/dev/null || {
    echo "    mkinitramfs failed, trying update-initramfs..."
    sudo chroot "$OUTPUT" bash -c "update-initramfs -u" 2>/dev/null || true
}

sudo umount "$OUTPUT/dev" 2>/dev/null || true
sudo umount "$OUTPUT/proc" 2>/dev/null || true
sudo umount "$OUTPUT/sys" 2>/dev/null || true

if [ -f "$OUTPUT/boot/initrd.img-${KVER}" ]; then
    echo "    initrd: $(sudo stat -c%s "$OUTPUT/boot/initrd.img-${KVER}") bytes"
else
    echo "    warning: initrd not generated"
fi

echo "==> Installing THISCLOUD components..."

sudo mkdir -p "$OUTPUT/etc/thpkg"
sudo mkdir -p "$OUTPUT/usr/lib/systemd/system"
sudo mkdir -p "$OUTPUT/usr/bin"
sudo mkdir -p "$OUTPUT/usr/share/first-run"
sudo mkdir -p "$OUTPUT/var/lib/extensions"
sudo mkdir -p "$OUTPUT/var/lib/thpkg/slots"
sudo mkdir -p "$OUTPUT/etc/systemd/system/multi-user.target.wants"

echo "$VERSION" | sudo tee "$OUTPUT/etc/thpkg/version" > /dev/null
echo "thiscloud-${VERSION}" | sudo tee "$OUTPUT/etc/hostname" > /dev/null

if [ -f "${SCRIPT_DIR}/../../target/release/thpkg" ]; then
    sudo cp "${SCRIPT_DIR}/../../target/release/thpkg" "$OUTPUT/usr/bin/thpkg"
    sudo chmod +x "$OUTPUT/usr/bin/thpkg"
    echo "    Installed thpkg binary"
fi

if [ -f "${SCRIPT_DIR}/../system/first-run/first-run.sh" ]; then
    sudo cp "${SCRIPT_DIR}/../system/first-run/first-run.sh" "$OUTPUT/usr/share/first-run/first-run.sh"
    sudo chmod +x "$OUTPUT/usr/share/first-run/first-run.sh"
fi

echo "==> Configuring systemd services..."

if [ -f "${SCRIPT_DIR}/../packages/thpkg/thpkg-booted-ok.service" ]; then
    sudo cp "${SCRIPT_DIR}/../packages/thpkg/thpkg-booted-ok.service" \
        "$OUTPUT/usr/lib/systemd/system/"
    sudo ln -sf /usr/lib/systemd/system/thpkg-booted-ok.service \
        "$OUTPUT/etc/systemd/system/multi-user.target.wants/"
fi

sudo tee "$OUTPUT/usr/lib/systemd/system/thiscloud-first-run.service" > /dev/null << 'UNIT'
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

echo "==> Configuring network..."

sudo mkdir -p "$OUTPUT/etc/systemd/network"
sudo tee "$OUTPUT/etc/systemd/network/20-wired.network" > /dev/null << 'NETWORK'
[Match]
Name=en* eth*

[Network]
DHCP=yes
NETWORK

sudo ln -sf /usr/lib/systemd/system/systemd-networkd.service \
    "$OUTPUT/etc/systemd/system/multi-user.target.wants/" 2>/dev/null || true
sudo ln -sf /usr/lib/systemd/system/systemd-resolved.service \
    "$OUTPUT/etc/systemd/system/multi-user.target.wants/" 2>/dev/null || true

echo "==> Creating bootloader config..."
sudo mkdir -p "$OUTPUT/boot/loader/entries"

sudo tee "$OUTPUT/boot/loader/loader.conf" > /dev/null << 'LOADER'
default thiscloud.conf
timeout 5
console-mode auto
editor no
LOADER

sudo tee "$OUTPUT/boot/loader/entries/thiscloud.conf" > /dev/null << EOF
title   THISCLOUD OS ${VERSION}
linux   /vmlinuz-${KVER}
initrd  /initrd.img-${KVER}
options root=/dev/sda3 rw
EOF

echo "==> Cleaning up..."
sudo chroot "$OUTPUT" bash -c "apt-get clean && rm -rf /var/lib/apt/lists/*" 2>/dev/null || true
sudo rm -rf "$OUTPUT/tmp/"* "$OUTPUT/var/tmp/"* 2>/dev/null || true

echo ""
echo "==> Rootfs created"
echo "    Path: $OUTPUT"
echo "    Size: $(du -sh "$OUTPUT" | awk '{print $1}')"
echo "    Kernel: ${KVER:-not found}"
