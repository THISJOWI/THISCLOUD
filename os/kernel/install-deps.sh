#!/usr/bin/env bash
# Install dependencies for the THISCLOUD Yocto image builder.
# Run on AlmaLinux 9 / RHEL 9 x86_64 as root.
#
#   sudo ./install-deps.sh
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
    echo "error: run as root, e.g. sudo $0" >&2
    exit 1
fi

echo "==> Installing Yocto build dependencies..."
dnf -y install \
    git \
    python3 \
    python3-pip \
    python3-devel \
    gcc \
    gcc-c++ \
    make \
    cmake \
    ninja-build \
    tar \
    gzip \
    bzip2 \
    xz \
    zstd \
    cpio \
    wget \
    curl \
    diffstat \
    texinfo \
    chrpath \
    socat \
    which \
    patch \
    bison \
    flex \
    perl \
    perl-ExtUtils-MakeMaker \
    gawk \
    findutils \
    unzip \
    parted \
    qemu-kvm \
    ed25519-devel \
    openssl-devel

echo "==> Installing Python packages for Yocto..."
pip3 install --upgrade pip
pip3 install \
    gitpython \
    repo

echo "==> Installing Rust for thpkg cross-compilation..."
if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
fi
rustup target add x86_64-unknown-linux-gnu

echo ""
echo "==> All dependencies installed."
echo "    You can now build the image:"
echo "    cd os/kernel"
echo "    ./build-image.sh"