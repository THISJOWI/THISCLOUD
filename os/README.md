# THISCLOUD OS

Server appliance OS built directly on the upstream Linux kernel. Source-built core (Yocto/OpenEmbedded), atomic A/B image updates, reproducible builds.

> **Status**: Phase F0 — Yocto prototype.

## Architecture

```
os/
  kernel/               Yocto layer (meta-thiscloud) + build scripts
    meta-thiscloud/     Yocto layer: kernel recipe, image recipe, thpkg recipe
    build-image.sh      Main build script (runs on AlmaLinux 9 x86_64)
    install-deps.sh     Install Yocto builder dependencies
    run-vm.sh           Boot built image in QEMU for testing
  packages/
    thpkg/              Rust source for thpkg (A/B slot manager)
  system/
    first-run/          First-run agent (role/IP/cluster config)
  build/
    scripts/            Legacy AlmaLinux ISO scripts (pre-migration)
  systemd/              Systemd unit files for thiscloud services
  calamares/            Legacy Calamares installer (pre-migration)
  branding/             Branding assets
```

## Disk layout (installed system)

```
ESP        (vfat, 512MiB)   systemd-boot + kernels + initrds
Slot A     (squashfs, ro)   rootfs: kernel + userspace + EL sysext (version N)
Slot B     (squashfs, ro)   same layout (version N+1 or previous before swap)
Data       (ext4, rw)       /var: config, etcd, state, logs, marketplace apps
```

- Each slot is self-contained: kernel + initrd + rootfs + EL sysext.
- Rootfs is read-only. Only `/var` (Data) and ESP are writable.
- Boot via systemd-boot with automatic rollback on failure.

## Package manager: thpkg

The `thpkg` binary manages A/B slots:

| Command | Description |
|---|---|
| `thpkg os-update` | Download + verify + write inactive slot + reboot |
| `thpkg status` | Show active slot, version, booted-ok state |
| `thpkg verify` | Verify slot signature and hashes |
| `thpkg booted-ok` | Healthcheck hook (systemd service after boot) |
| `thpkg init` | First-run: write config, run `thiscloud init` |

State: `/var/lib/thpkg/`

## Update flow

```
thpkg os-update
  → fetch manifest → verify hash → download slot (kernel + initrd + rootfs)
  → write to inactive slot → set BootNext → reboot
  → initrd validates signature (fail → previous slot)
  → thpkg-booted-ok runs healthcheck (fail → reboot to previous slot)
```

## EL layer (build-time)

OVN/OVS, DRBD, etcd, nginx are materialized from EL RPMs into a
**systemd-sysext** squashfs image during the build. The EL builder
(AlmaLinux 9) uses `dnf` to fetch RPMs, then repackages them into
`el-layer-<kernel>-<ver>.sysext` shipped inside each slot.

At runtime: **no `dnf`**. 100% immutable.

## Build the image

```sh
# 1. Install builder dependencies (AlmaLinux 9 x86_64 only)
sudo os/kernel/install-deps.sh

# 2. Build the image
cd os/kernel
./build-image.sh

# 3. Test in QEMU
./run-vm.sh
```

## Services on the installed system

| Service | Port | Description |
|---|---|---|
| `thiscloudd.service` | 8080 | Rust hypervisor daemon |
| `thiscloud-api.service` | 8081 | Go orchestrator API server |
| `thiscloud-webui.service` | 3000 | Next.js web UI server |
| `nginx.service` | 80 | Reverse proxy → web UI on port 3000 |
| `etcd.service` | 2379 | Key-value store for cluster state |

## Migration phases

- **F0**: Yocto prototype — kernel mainline + systemd + minimal boot in VM
- **F1**: Full userspace + thpkg A/B management
- **F2**: EL layer (OVN/OVS/DRBD/etcd/nginx) + thiscloud services
- **F3**: Installer ISO + first-run agent
- **F4**: CI pipeline + VM tests
- **F5**: Cutover (retire Alma runtime)
