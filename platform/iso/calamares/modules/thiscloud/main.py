#!/usr/bin/env python3
"""THISCLOUD Calamares job module — applies node config to the target.

Runs after partition/mount (target available at rootMountPoint),
before/around bootloader. Reads GlobalStorage keys set by thiscloudqml.
"""
import libcalamares

from thiscloud_logic import (is_valid_ip, is_valid_role,
                             sanitize_cluster_name, services_for_role,
                             write_config, build_init_args)

_ = lambda s: s  # noqa: E731 — translation is optional for this module


def pretty_name():
    return _("THISCLOUD configuration")


def run():
    gs = libcalamares.globalstorage
    root = gs.value("rootMountPoint")
    if not root:
        return (_("No root mount point"),
                _("rootMountPoint was not set; mount module did not run."))

    role = str(gs.value("thiscloudRole") or "worker").strip()
    cluster = sanitize_cluster_name(str(gs.value("thiscloudClusterName") or "thiscloud"))
    ip = str(gs.value("thiscloudNodeIp") or "127.0.0.1").strip()
    iface = str(gs.value("thiscloudInterface") or "eth0").strip()

    if not is_valid_role(role):
        return (_("Invalid node role"),
                _("thiscloudRole must be 'master' or 'worker', got '{role}'.").format(role=role))
    if not is_valid_ip(ip):
        return (_("Invalid node IP"),
                _("thiscloudNodeIp '{ip}' is not a valid IP address.").format(ip=ip))

    libcalamares.utils.debug("Writing /etc/thiscloud/config.toml")
    write_config(root, cluster, ip, role, iface)

    libcalamares.utils.debug("Running thiscloud init")
    r = libcalamares.utils.target_env_call(build_init_args(ip, role))
    if r != 0:
        return (_("thiscloud init failed"),
                _("`thiscloud init --ip {ip} --role {role}` exited {code}.").format(ip=ip, role=role, code=r))

    for unit in services_for_role(role):
        libcalamares.utils.target_env_call(
            ["/usr/bin/systemctl", "enable", unit])

    libcalamares.globalstorage.insert("thiscloudClusterName", cluster)
    return None