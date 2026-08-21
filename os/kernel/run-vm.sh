#!/usr/bin/env bash
# Boot the THISCLOUD image in QEMU for testing.
#
#   ./run-vm.sh [path-to-image]
#
# Defaults to the latest built image in build/tmp/deploy/images/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}/build"
DEPLOY_DIR="$BUILD_DIR/tmp/deploy/images"
MACHINE="${MACHINE:-qemu-x86-64}"

IMAGE="${1:-}"
if [ -z "$IMAGE" ]; then
    # Find the latest rootfs image
    IMAGE=$(find "$DEPLOY_DIR/$MACHINE" -name "thiscloud-image-*.ext4" -o -name "thiscloud-image-*.wic" 2>/dev/null | head -1)
    if [ -z "$IMAGE" ]; then
        echo "error: no image found in $DEPLOY_DIR/$MACHINE"
        echo "  Build first: ./build-image.sh"
        exit 1
    fi
fi

echo "==> Booting THISCLOUD in QEMU"
echo "    Image: $IMAGE"

# Find kernel and initrd
KERNEL=$(find "$DEPLOY_DIR/$MACHINE" -name "bzImage" | head -1)
INITRD=$(find "$DEPLOY_DIR/$MACHINE" -name "core-image-*.cpio.gz" | head -1)

if [ -z "$KERNEL" ] || [ -z "$INITRD" ]; then
    echo "error: kernel or initrd not found in $DEPLOY_DIR/$MACHINE"
    exit 1
fi

echo "    Kernel: $KERNEL"
echo "    Initrd: $INITRD"

# Boot QEMU with:
# - 2 CPU cores, 2GB RAM (hypervisor host minimum)
# - Serial console on stdio
# - virtio-net for networking
# - AHCI controller for the rootfs
qemu-system-x86_64 \
    -enable-kvm \
    -m 2048 \
    -smp 2 \
    -kernel "$KERNEL" \
    -initrd "$INITRD" \
    -append "console=ttyS0 root=/dev/vda rw" \
    -drive file="$IMAGE",if=virtio,format=raw \
    -netdev user,id=net0,hostfwd=tcp::8080-:8080,hostfwd=tcp::2222-:22 \
    -device virtio-net-pci,netdev=net0 \
    -nographic \
    -no-reboot

QEMU_EXIT=$?
echo ""
echo "==> VM exited (exit code: $QEMU_EXIT)"