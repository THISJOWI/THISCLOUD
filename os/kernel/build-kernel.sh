#!/usr/bin/env bash
# build-kernel.sh — Compile vanilla Linux kernel from kernel.org
# Usage: ./build-kernel.sh [--version 6.15.6] [--output build/kernel]
set -euo pipefail

VERSION="${VERSION:-6.15.6}"
OUTPUT="${OUTPUT:-build/kernel}"
JOBS="${JOBS:-$(nproc 2>/dev/null || echo 4)}"
CACHE_DIR="${CACHE_DIR:-/tmp/thcloud-kernel-cache}"

# ── Parse args ──────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version) VERSION="$2"; shift 2 ;;
        --output)  OUTPUT="$2"; shift 2 ;;
        --jobs)    JOBS="$2"; shift 2 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

TARBALL="linux-${VERSION}.tar.xz"
URL="https://cdn.kernel.org/pub/linux/kernel/v6.x/${TARBALL}"

echo "==> Building vanilla Linux kernel ${VERSION}"
echo "    Output: ${OUTPUT}"
echo "    Jobs:   ${JOBS}"

mkdir -p "$CACHE_DIR"
mkdir -p "$OUTPUT"

# ── 1. Download kernel source ──────────────────────────────────────

SRC_DIR="${CACHE_DIR}/linux-${VERSION}"

if [ -d "$SRC_DIR" ]; then
    echo "    Using cached source: ${SRC_DIR}"
else
    echo "==> Downloading kernel ${VERSION}..."
    if [ ! -f "${CACHE_DIR}/${TARBALL}" ]; then
        curl -L -o "${CACHE_DIR}/${TARBALL}" "$URL"
    fi
    echo "==> Extracting..."
    tar -xf "${CACHE_DIR}/${TARBALL}" -C "$CACHE_DIR"
fi

# Resolve to absolute paths before cd
OUTPUT="$(cd "$OUTPUT" 2>/dev/null && pwd || (mkdir -p "$OUTPUT" && cd "$OUTPUT" && pwd))"
CACHE_DIR="$(cd "$CACHE_DIR" 2>/dev/null && pwd || (mkdir -p "$CACHE_DIR" && cd "$CACHE_DIR" && pwd))"

# ── 2. Configure kernel ───────────────────────────────────────────

echo "==> Configuring kernel..."
cd "$SRC_DIR"

# Start with defconfig
make defconfig

# Enable required options for ThisCloud OS (server appliance)
cat >> .config << 'KERNEL_CONFIG'
# Storage
CONFIG_EXT4_FS=y
CONFIG_XFS_FS=m
CONFIG_VFAT_FS=y
CONFIG_FAT_DEFAULT_UTF8=y
CONFIG_NTFS3_FS=m

# Boot
CONFIG_EFI=y
CONFIG_EFI_STUB=y
CONFIG_DMI=y

# Virtio (for VMs)
CONFIG_VIRTIO=y
CONFIG_VIRTIO_PCI=y
CONFIG_VIRTIO_BLK=y
CONFIG_VIRTIO_NET=y
CONFIG_VIRTIO_CONSOLE=y
CONFIG_VIRTIO_MMIO=y

# SATA/NVMe
CONFIG_ATA=y
CONFIG_ATA_PIIX=y
CONFIG_SATA_AHCI=y
CONFIG_NVME_CORE=y
CONFIG_BLK_DEV_NVME=y

# Network
CONFIG_E1000=y
CONFIG_E1000E=y
CONFIG_IGB=y
CONFIG_IGBVF=y
CONFIG_NETDEVICES=y
CONFIG_NET_CORE=y
CONFIG_INET=y

# USB
CONFIG_USB=y
CONFIG_USB_XHCI_HCD=y
CONFIG_USB_EHCI_HCD=y
CONFIG_USB_STORAGE=y

# Filesystem utils
CONFIG_TMPFS=y
CONFIG_TMPFS_POSIX_ACL=y
CONFIG_DEVTMPFS=y
CONFIG_DEVTMPFS_MOUNT=y

# systemd requirements
CONFIG_CGROUPS=y
CONFIG_CGROUP_FREEZER=y
CONFIG_CGROUP_DEVICE=y
CONFIG_CPUSETS=y
CONFIG_MEMCG=y
CONFIG_KEYS=y
CONFIG_SECCOMP=y
CONFIG_SECCOMP_FILTER=y
CONFIG_BPF_SYSCALL=y
CONFIG_USERFAULTFD=y
CONFIG_FHANDLE=y
CONFIG_SIGNALFD=y
CONFIG_TIMERFD=y
CONFIG_EVENTFD=y
CONFIG_EPOLL=y
CONFIG_INOTIFY_USER=y
CONFIG_MACVLAN=y
CONFIG_VLAN_8021Q=y
CONFIG_VXLAN=y
CONFIG_BRIDGE=m
CONFIG_NETFILTER=y
CONFIG_NF_CONNTRACK=m
CONFIG_NF_NAT=m
CONFIG_IPTABLES=m
CONFIG_IP6_NTABLES=m
CONFIG_IP_NF_FILTER=m
CONFIG_IP_NF_NAT=m
CONFIG_IP6_NF_FILTER=m

# Security
CONFIG_SECURITY=y
CONFIG_SECURITY_NETWORK=y

# Compression (for initrd)
CONFIG_RD_GZIP=y
CONFIG_RD_XZ=y
CONFIG_RD_LZ4=y
KERNEL_CONFIG

# Enable required options and autoresolve dependencies
make olddefconfig

# ── 3. Build kernel ───────────────────────────────────────────────

echo "==> Compiling kernel (this takes ~20 min)..."
make -j"$JOBS"

echo "==> Building modules..."
make -j"$JOBS" modules

# ── 4. Install to output ─────────────────────────────────────────

echo "==> Installing kernel to ${OUTPUT}..."
mkdir -p "$OUTPUT/boot"
mkdir -p "$OUTPUT/lib/modules/${VERSION}"

# Copy kernel
cp arch/x86/boot/bzImage "$OUTPUT/boot/vmlinuz-${VERSION}"
cp System.map "$OUTPUT/boot/System.map-${VERSION}"
cp .config "$OUTPUT/boot/config-${VERSION}"

# Install modules
make INSTALL_MOD_PATH="$OUTPUT" modules_install

# ── 5. Generate module dependency ─────────────────────────────────

echo "==> Generating module dependencies..."
depmod -a "$VERSION" -b "$OUTPUT"

echo "==> Kernel build complete!"
echo "    Kernel:  $OUTPUT/boot/vmlinuz-${VERSION}"
echo "    Modules: $OUTPUT/lib/modules/${VERSION}/"
ls -lh "$OUTPUT/boot/vmlinuz-${VERSION}"
