#!/usr/bin/env bash
# Create a complete THISCLOUD OS rootfs using debootstrap.
# Uses a pre-built vanilla Linux kernel (from build-kernel.sh).
#
# Usage:
#   ./build-rootfs.sh --output /path/to/rootfs --version 0.4.0 \
#       --kernel-version 6.15.6 --kernel-dir /path/to/kernel-output
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION="${VERSION:-0.1.0}"
OUTPUT=""
SUITE="bookworm"
KVER=""
KERNEL_DIR=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --output) OUTPUT="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        --suite) SUITE="$2"; shift 2 ;;
        --kernel-version) KVER="$2"; shift 2 ;;
        --kernel-dir) KERNEL_DIR="$2"; shift 2 ;;
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
    --include=sudo,vim,nano,less,psmisc,procps,iproute2,ethtool \
    --include=net-tools,openssh-server,openssh-client,curl,wget \
    --include=e2fsprogs,dosfstools,parted,gdisk,blkid \
    --include=syslinux,syslinux-common,isolinux \
    "$SUITE" "$OUTPUT" http://deb.debian.org/debian

# ── 2. Install vanilla kernel ─────────────────────────────────────

echo "==> Installing vanilla Linux kernel..."

if [ -n "$KERNEL_DIR" ] && [ -d "$KERNEL_DIR" ]; then
    echo "    Using pre-built kernel from: $KERNEL_DIR"

    # Copy kernel
    if [ -f "$KERNEL_DIR/boot/vmlinuz-${KVER}" ]; then
        sudo cp "$KERNEL_DIR/boot/vmlinuz-${KVER}" "$OUTPUT/boot/vmlinuz-${KVER}"
        echo "    Installed vmlinuz-${KVER}"
    else
        echo "    error: kernel not found at $KERNEL_DIR/boot/vmlinuz-${KVER}"
        exit 1
    fi

    # Copy System.map and config
    [ -f "$KERNEL_DIR/boot/System.map-${KVER}" ] && \
        sudo cp "$KERNEL_DIR/boot/System.map-${KVER}" "$OUTPUT/boot/System.map-${KVER}"
    [ -f "$KERNEL_DIR/boot/config-${KVER}" ] && \
        sudo cp "$KERNEL_DIR/boot/config-${KVER}" "$OUTPUT/boot/config-${KVER}"

    # Copy modules
    if [ -d "$KERNEL_DIR/lib/modules/${KVER}" ]; then
        sudo mkdir -p "$OUTPUT/lib/modules"
        sudo cp -a "$KERNEL_DIR/lib/modules/${KVER}" "$OUTPUT/lib/modules/"
        echo "    Installed modules ($(du -sh "$OUTPUT/lib/modules/${KVER}" | awk '{print $1}'))"
    fi
else
    echo "    No --kernel-dir provided, falling back to Debian kernel"
    sudo chroot "$OUTPUT" bash -c "apt-get update && apt-get install -y --no-install-recommends linux-image-amd64 systemd-sysv" 2>/dev/null || true
    KVER=$(ls "$OUTPUT/boot/vmlinuz-"* 2>/dev/null | head -1 | xargs basename 2>/dev/null | sed 's/^vmlinuz-//')
fi

# Verify kernel exists
if [ ! -f "$OUTPUT/boot/vmlinuz-${KVER}" ]; then
    echo "error: vmlinuz-${KVER} not found in $OUTPUT/boot/"
    ls -la "$OUTPUT/boot/"
    exit 1
fi

# ── 3. Generate initrd ─────────────────────────────────────────────

echo "==> Generating initrd for ${KVER}..."

# Mount necessary filesystems for chroot
sudo mount --bind /dev "$OUTPUT/dev" 2>/dev/null || true
sudo mount -t proc proc "$OUTPUT/proc" 2>/dev/null || true
sudo mount -t sysfs sysfs "$OUTPUT/sys" 2>/dev/null || true

# Generate initrd using mkinitramfs (Debian's tool)
sudo chroot "$OUTPUT" bash -c "mkinitramfs -o /boot/initrd.img-${KVER} ${KVER}" 2>/dev/null || {
    echo "    mkinitramfs failed, trying manual initrd..."
    # Fallback: create minimal initrd with needed modules
    sudo chroot "$OUTPUT" bash -c "
        mkdir -p /tmp/initrd-\${KVER}
        cd /tmp/initrd-\${KVER}
        mkdir -p bin lib lib64 sbin etc proc sys dev tmp

        # Copy busybox if available
        if command -v busybox >/dev/null 2>&1; then
            cp \$(which busybox) bin/busybox
            for cmd in sh mount umount mkdir cp mv ln chmod sync cat echo sleep; do
                ln -sf busybox bin/\$cmd
            done
        fi

        # Copy essential modules
        for mod in virtio_pci virtio_blk virtio_net ext4 crc16 mbcache jbd2; do
            find /lib/modules/${KVER} -name \"\${mod}.ko*\" -exec cp {} lib/ \; 2>/dev/null || true
        done

        # Create init script
        cat > init << 'INIT'
#!/bin/sh
mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev
exec /sbin/init
INIT
        chmod +x init

        # Pack
        find . | cpio -o -H newc 2>/dev/null | gzip > /boot/initrd.img-${KVER}
        rm -rf /tmp/initrd-\${KVER}
    " 2>/dev/null || echo "    warning: manual initrd also failed"
}

# Unmount
sudo umount "$OUTPUT/dev" 2>/dev/null || true
sudo umount "$OUTPUT/proc" 2>/dev/null || true
sudo umount "$OUTPUT/sys" 2>/dev/null || true

# Verify initrd
if [ -f "$OUTPUT/boot/initrd.img-${KVER}" ]; then
    echo "    initrd: $(sudo stat -c%s "$OUTPUT/boot/initrd.img-${KVER}") bytes"
else
    echo "    warning: initrd not generated"
fi

# ── 4. Set up THISCLOUD components ─────────────────────────────────

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

# ── 5. systemd services ────────────────────────────────────────────

echo "==> Configuring systemd services..."

# thpkg booted-ok service
if [ -f "${SCRIPT_DIR}/../packages/thpkg/thpkg-booted-ok.service" ]; then
    sudo cp "${SCRIPT_DIR}/../packages/thpkg/thpkg-booted-ok.service" \
        "$OUTPUT/usr/lib/systemd/system/"
    sudo ln -sf /usr/lib/systemd/system/thpkg-booted-ok.service \
        "$OUTPUT/etc/systemd/system/multi-user.target.wants/"
fi

# first-run service
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

# ── 6. Network configuration ────────────────────────────────────────

echo "==> Configuring network..."

sudo mkdir -p "$OUTPUT/etc/systemd/network"
sudo tee "$OUTPUT/etc/systemd/network/20-wired.network" > /dev/null << 'NETWORK'
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

# ── 7. Bootloader config (systemd-boot) ────────────────────────────

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

# ── 8. Clean up ─────────────────────────────────────────────────────

echo "==> Cleaning up..."
sudo chroot "$OUTPUT" bash -c "apt-get clean && rm -rf /var/lib/apt/lists/*" 2>/dev/null || true
sudo rm -rf "$OUTPUT/tmp/"* "$OUTPUT/var/tmp/"* 2>/dev/null || true

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
