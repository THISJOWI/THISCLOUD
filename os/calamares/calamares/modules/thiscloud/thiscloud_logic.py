"""Pure logic for the THISCLOUD Calamares job module (no libcalamares).

Kept importable outside Calamares so it is unit-testable on any host.
"""
import ipaddress
import os
import re


def is_valid_ip(value):
    try:
        ipaddress.ip_address(value.strip())
        return True
    except (ValueError, AttributeError):
        return False


def is_valid_role(value):
    return value.strip() in ("master", "worker")


def sanitize_cluster_name(value):
    """Lowercase; whitespace becomes single hyphens, other junk is dropped."""
    value = value.strip().lower()
    value = re.sub(r"\s+", "-", value)
    return re.sub(r"[^a-z0-9-]+", "", value)


def render_config_toml(cluster, ip, role, iface):
    cluster = sanitize_cluster_name(cluster)
    return (
        "# THISCLOUD node configuration (written by the installer)\n"
        f'cluster_name = "{cluster}"\n'
        f'node_ip = "{ip}"\n'
        f'role = "{role}"\n'
        f'interface = "{iface}"\n'
    )


def write_config(root, cluster, ip, role, iface):
    """Write config.toml under root/etc/thiscloud/. Returns the path."""
    cfg_dir = os.path.join(root, "etc", "thiscloud")
    os.makedirs(cfg_dir, exist_ok=True)
    path = os.path.join(cfg_dir, "config.toml")
    with open(path, "w") as f:
        f.write(render_config_toml(cluster, ip, role, iface))
    return path


def services_for_role(role):
    """Systemd units to enable for the given node role."""
    base = ["thiscloudd.service", "thiscloud-webui.service",
            "thiscloud-ports.service", "thiscloud-web-port.service"]
    if role == "master":
        base.append("thiscloud-api.service")
    return sorted(set(base))


def build_init_args(ip, role):
    """Argv for `thiscloud init` in the target chroot."""
    return ["/usr/bin/thiscloud", "init", "--ip", ip.strip(), "--role", role.strip()]