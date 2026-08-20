# THISCLOUD Kernel-Based OS — Design

**Date:** 2026-08-20
**Status:** Approved
**Goal:** Migrate THISCLOUD off the AlmaLinux 9 base onto a purpose-built server OS directly on the upstream Linux kernel: source-built core (Yocto), atomic A/B image updates, reproducible builds, minimal footprint. The system package manager is `thpkg`.

## Problem

The current ISO pipeline (`platform/iso/`) builds a customized AlmaLinux 9 live ISO (livemedia-creator + Calamares) that installs THISCLOUD plus RPM dependencies (OVN/OVS, DRBD, Linstor, etcd, nginx, qemu-kvm). This couples the product to an upstream distro (RHEL/Alma) for:

- **Updates**: installed systems pull from GitHub Releases; rollback relies on a fragile `dnf downgrade`.
- **Footprint**: full EL userspace, toolchains in the build, graphical live session (Xorg/openbox/Calamares) shipped as installer.
- **Independence**: release cadence, lifecycle, security patch window and branding are all inherited from Alma/RHEL.
- **Reproducibility**: ISO assembly is not deterministic; builds depend on repo state of the day.

## Requirements

1. **Independence** — no upstream distro underneath. Kernel = upstream Linux (vanilla, unpatched by a vendor distro).
2. **Small footprint** — minimal server appliance image; no toolchains/compilers on target; no graphical installer.
3. **Atomic, reliable updates** — whole-OS image update with hardware-guaranteed rollback.
4. **Reproducibility** — same source pins → same image, verified by hashes.
5. **Keep existing integrations** — thiscloudd backends (ovn-nbctl, drbd-utils, linstor) keep working; OVN/OVS/DRBD/Linstor/etcd/nginx remain available; Web UI / Go API / daemon unchanged.

## Decisions (locked)

| Decision | Choice |
|---|---|
| Kernel | Upstream Linux mainline (vanilla), reduced config for hypervisor host |
| Init | systemd (trimmed) |
| Build system | Yocto/OpenEmbedded (source-built core) |
| Package model | Atomic image A/B (slots) |
| EL dependencies | Hybrid: core from source; difficult deps (OVN/OVS, DRBD, Linstor, etcd, nginx) materialized at **build-time** from EL RPMs into a **systemd-sysext** image — no `dnf` at runtime |
| Package manager | `thpkg` |
| DRBD | Kernel module built from source against our kernel (ELRepo kmod will not load on a custom kernel); tools shipped in the EL layer |
| OVS | Userspace datapath (netdev), no kernel module |
| SELinux | Off (hardening via read-only rootfs + systemd; avoids friction with EL layer) |
| Boot | UEFI-only, systemd-boot |
| Linstor | Kept in EL layer, **out of MVP** |
| Own components | Baked into the slot image, versioned with the slot (hot-update axis deferred, extensible) |

## Architecture

GPT + UEFI disk layout:

```
ESP        (vfat, 512MiB)   systemd-boot + kernels + initrds
Slot A     (squashfs, ro)   full rootfs: kernel + userspace + EL sysext (version N)
Slot B     (squashfs, ro)   same layout (version N+1 or previous before swap)
Data       (ext4/xfs, rw)   /var: thiscloud config, etcd data, state, logs, marketplace apps
```

- Each slot is **self-contained**: kernel + initrd + rootfs + EL sysext, so kernel↔modules (DRBD) are always matched.
- **Boot**: systemd-boot with two entries (ThisCloud A / ThisCloud B) plus automatic rollback entry.
- **Immutability**: rootfs mounted read-only. Only `/var` (Data) and the ESP are writable. No runtime modification of the OS.
- **EL layer** (`el-layer-<kernel>-<ver>.sysext`): squashfs systemd-sysext merged over `/usr` at boot. Signed, pinned inside the slot.

## Package manager: `thpkg`

Single binary, Rust. Commands:

- `thpkg os-update` — read remote manifest, detect inactive slot, download signed slot, verify hashes + Ed25519 signature, write squashfs to inactive slot, set `systemd-boot` BootNext, reboot.
- `thpkg status` — slot versions, active/inactive, booted-ok state, EL layer version.
- `thpkg verify` — verify signature/hashes of current and staged slots.
- `thpkg booted-ok` — systemd hook; runs healthcheck of critical services after boot.

State recorded in `/var/lib/thpkg`.

### Update / rollback flow

```
thpkg os-update
  1. read remote manifest → detect inactive slot
  2. download signed slot, verify hashes + Ed25519
  3. write squashfs to inactive slot
  4. set BootNext = new slot
  5. reboot
  6. boot: initrd validates signature; on failure → boot previous slot
  7. booted-ok hook: healthcheck critical services (thiscloudd, go-api, web-ui, ovn, etcd)
  8a. OK → mark slot booted-ok, keep previous as rollback
  8b. FAIL → reboot → previous slot
```

- Rollback requires **no state restore**: rootfs immutable, `/var` shared by both slots (config/etcd survive the swap).
- Boot watchdog (N-attempt timeout) as an additional safety net.

### Own components

thiscloudd, thiscloud-cli, go-api, web-ui are baked into the slot image and versioned with it. Marketplace apps are not host packages (they run in VMs/containers) and do not touch `thpkg`. The current `thiscloud update` (GitHub Releases → hot binary swap) is refactored into `thpkg os-update` (full slot). A runtime sysext axis (updating own components without reboot) is a deferred extension point — not built in v1.

## Userspace (inside the image)

- **Kernel**: Linux mainline, reduced config: KVM/vhost (cloud-hypervisor), virtio, NVMe/SATA/SAS, network, ipv6, filesystems (ext4, xfs, squashfs, overlayfs), DRBD module.
- **PID 1**: systemd (trimmed).
- **libc**: glibc (EL RPM binaries in the sysext assume glibc).
- **Core packages**: coreutils, openssh (sole shell access), openssl, ca-certificates, iproute2, nftables (OVN relies on openvswitch), sudo, util-linux.
- **thiscloud services**: thiscloudd, go-api, web-ui, etcd, nginx (nginx ships in the EL layer).
- **Appliance hardening by default**: read-only rootfs, default firewall only on ports 80/8080/8081/2379, no remote root shell, SELinux off.
- **Not in the image**: compilers, toolchains, node, npm, make. Build toolchain lives only on the builder.

**Boot sequence**: systemd-boot → minimal initramfs (mounts squashfs + EL sysext + `/var`) → systemd → thiscloud services.

## EL layer (build-time)

- EL builder (Alma 9) uses dnf to download the exact RPMs from NFV SIG / ELRepo / LINBIT / AppStream, unpack and repackage them as `el-layer-<kernel>-<ver>.sysext` (squashfs).
- DRBD kmod is **not** taken from ELRepo RPM — built from source in Yocto against our kernel and shipped in the sysext with the userspace tools.
- Kernel↔sysext match is guaranteed because both are built together and shipped in the same slot.
- Daemon backends (ovn-nbctl, linstor, drbd-utils) keep working unchanged — only their on-disk provenance changes.

## Installer and first run

- **Remove** Calamares, KPMcore, Qt (the largest build burden of the current pipeline).
- New installer = **minimal flashing ISO**: partitions GPT (ESP + Slot A + Slot B + Data), writes Slot A, installs systemd-boot, reboots. No graphical wizard — it writes an image, not a config.
- **First run** (one-shot service on installed system): configures role / cluster / IP / interface / session secret. The configuration logic of the current Calamares thiscloud module is **reused** as an agent/script, not discarded.

## Build pipeline and CI

- Two builders:
  1. **Yocto builder** (Linux): kernel mainline + userspace + DRBD kmod → rootfs squashfs + slot.
  2. **EL builder** (Alma 9): dnf → EL sysext.
- CI (GitHub Actions, rework of `release.yml`/`iso.yml`): releases produce **signed slots + manifest**, not RPMs. `iso.yml` → flashing ISO.
- **Reproducibility**: Yocto layer + source lockfile (hashes); manifest lists hashes of every component.
- Rust/Go/Node toolchains stay on the builder.

## Migration phases

1. **F0 — Yocto prototype**: kernel mainline + systemd + minimal boot in a VM (validate Yocto produces a bootable image).
2. **F1 — Userspace + `thpkg` v1**: full userspace, A/B partition layout, `thpkg os-update` + rollback + booted-ok.
3. **F2 — EL layer + thiscloud services**: sysext (OVN/OVS/DRBD/etcd/nginx), thiscloudd/go-api/web-ui in slot, healthchecks.
4. **F3 — Installer + first run**: flashing ISO + configuration agent.
5. **F4 — CI + releases**: release pipeline with signed slots, VM tests (migrate calamares/ tests to thpkg/installer tests).
6. **F5 — Cutover**: retire the Alma runtime flow; the EL builder only produces the sysext.

## Testing and verification

- **F0**: image boots in VM, systemd up, initramfs mounts rootfs.
- **F1**: `thpkg` tests — slot swap, forced rollback, rejected corrupted signature, booted-ok ok/fail paths.
- **F2**: post-boot healthchecks for OVN/OVS/DRBD/etcd; daemon backends against the sysext.
- **F3**: installer end-to-end in VM (partition → boot → first-run → services green).
- **F4**: existing test suites (Rust/Go/web-ui/openapi) wired into the new pipeline; calamares tests migrated to thpkg/installer tests.
- CI gates: each release verifies signed slot + boot in a VM.

## Out of scope (deferred)

- Runtime sysext axis for hot-updating own components without reboot.
- BIOS/legacy boot support (UEFI-only).
- Linstor as a shipped EL-layer component (the F2 sysext excludes it; it can be added later without architecture changes).
- SELinux/AppArmor enforcement.