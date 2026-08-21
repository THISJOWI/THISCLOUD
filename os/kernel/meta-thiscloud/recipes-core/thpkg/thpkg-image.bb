# thiscloud-image — minimal server image for THISCLOUD
# Builds: kernel mainline + systemd + thpkg + minimal server userspace

require recipes-core/core-image.bb

SUMMARY = "THISCLOUD Hypervisor OS"
DESCRIPTION = "Minimal server image with systemd, kernel mainline, and thpkg"
LICENSE = "MIT"

# Image features — minimal server, no GUI
IMAGE_FEATURES = " \
    ssh-server-openssh \
    read-only-rootfs \
    systemd-boot \
"

IMAGE_INSTALL:append = " \
    thpkg \
    thpkg-service \
    iproute2 \
    iptables \
    nftables \
    sudo \
    util-linux \
    coreutils \
    openssl \
    ca-certificates \
    openssh \
    systemd-analyze \
"

# Remove unnecessary packages to reduce footprint
IMAGE_INSTALL:remove = " \
    packagegroup-core-ssh-openssh \
"

# Root filesystem type
IMAGE_FSTYPES = "squashfs ext4"

# Do not generate additional images
IMAGE_ROOTFS_SIZE = "4096"

# Boot with systemd-boot
WKS_FILE = "thiscloud.wks"

# Package images to generate
IMAGE_ROOTFS_EXTRA_SPACE = "0"
