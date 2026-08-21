# thpkg — THISCLOUD package manager
# Builds from os/packages/thpkg/ in the repo

SUMMARY = "THISCLOUD package manager — A/B slot management"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE;md5=MIT"

# Source lives in the repo root under os/packages/thpkg
FILESEXTRAPATHS:prepend := "${THISDIR}/files:"
SRC_URI = " \
    file://thpkg-${PV}.tar.gz \
"

# Cargo build via the cargo class
inherit cargo

S = "${WORKDIR}/thpkg-${PV}"

# Dependencies (from Cargo.toml)
CARGO_INSTALL_PATH = "os/packages/thpkg"

# Install to standard system paths
do_install:append() {
    install -d ${D}${bindir}
    install -m 0755 ${B}/target/release/thpkg ${D}${bindir}/thpkg

    # Install systemd service
    install -d ${D}${systemd_system_unitdir}
    install -m 0644 ${S}/thpkg.service ${D}${systemd_system_unitdir}/thpkg.service

    # Install healthcheck service (runs after boot)
    install -m 0644 ${S}/thpkg-booted-ok.service ${D}${systemd_system_unitdir}/thpkg-booted-ok.service
}

FILES:${PN} = " \
    ${bindir}/thpkg \
    ${systemd_system_unitdir}/thpkg.service \
    ${systemd_system_unitdir}/thpkg-booted-ok.service \
"

SYSTEMD_SERVICE:${PN} = "thpkg.service thpkg-booted-ok.service"