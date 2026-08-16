# THISCLOUD ISO Build Tooling

Custom AlmaLinux 9 ISO that auto-installs THISCLOUD (daemon + CLI + Go API + Web UI) together with
all runtime dependencies: cloud-hypervisor, OVN/OVS, DRBD, Linstor and etcd.

> **Important**: the ISO itself cannot be produced on an arm64 macOS host. This
> directory ships the complete, ready-to-run build pipeline. Execute it on an
> AlmaLinux 9 **x86_64** builder (bare metal or VM). macOS can only produce the
> cross-compiled binaries.

## Layout

```
iso/
  kickstart/                  (removed — installer is now Calamares-based)
  calamares/                  custom Calamares installer (see Installer section below)
  scripts/cross-compile.sh    thiscloudd/thiscloud → x86_64-unknown-linux-gnu
  scripts/prepare-rpm.sh      cargo-generate-rpm before-build hook (copies binaries)
  scripts/build-iso.sh        full pipeline: cross → RPM → go-api → web-ui → repo → live ISO
  scripts/fetch-deps.sh       stage cloud-hypervisor + dependency RPMs + systemd units
  scripts/make-repo.sh        build local RPM repo metadata
  systemd/thiscloudd.service   Rust daemon systemd unit
  systemd/thiscloud-api.service  Go API server systemd unit
  systemd/thiscloud-webui.service  Next.js web UI systemd unit
  repo/                        assembled RPM repository + binaries
    thiscloud/                 RPM repository (repodata + packages)
    cloud-hypervisor           static hypervisor binary
    thiscloud-api              Go API server binary
    web-ui/                    pre-built Next.js standalone output
    systemd/                   systemd unit files for the ISO target
```

## Pipeline

1. **Cross-compile** — `scripts/cross-compile.sh`
   `rustup target add x86_64-unknown-linux-gnu && cargo build --release --target x86_64-unknown-linux-gnu`

2. **RPM packaging** — via `cargo-generate-rpm`
   Metadata lives in each crate's `Cargo.toml` `[package.metadata.generate-rpm]`.
   - `thiscloudd` → `/usr/sbin/thiscloudd` + systemd unit
   - `thiscloud-cli` → `/usr/bin/thiscloud`
   `cargo generate-rpm --target x86_64-unknown-linux-gnu -p thiscloudd`
   `cargo generate-rpm --target x86_64-unknown-linux-gnu -p thiscloud-cli`

3. **Go API build** — `build-iso.sh` step 3
   `CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o iso/repo/thiscloud-api ./cmd/api-server`

4. **Web UI build** — `build-iso.sh` step 4
   `cd web-ui && npm install && npm run build`
   Produces standalone Next.js output in `iso/repo/web-ui/`.

5. **Dependency fetch** — `scripts/fetch-deps.sh`
   Downloads cloud-hypervisor static binary, OVN/OVS + DRBD + Linstor RPMs,
   nginx, qemu-kvm, and stages systemd service files.

6. **Local repository** — `scripts/build-iso.sh`
   `createrepo_c iso/repo/thiscloud` (repo referenced from the Calamares live
   flow via a local repo entry).

7. **Installer + ISO build** — `scripts/build-iso.sh` step 8
   `calamares/scripts/build-live-iso.sh` compiles Calamares + KPMcore from
   source, packages them as RPMs into `iso/repo`, then assembles the **live**
   ISO with `livemedia-creator --make-iso --no-virt` using
   `calamares/live/live.ks`. Produces `ThisCloud-<VERSION>-x86_64.iso`.

8. **Test in a VM** — boot the ISO in qemu/libvirt; it boots a live Xorg +
   openbox session that auto-launches Calamares, which installs THISCLOUD
   (partitions via KPMcore, copies filesystem, installs GRUB, runs
   `thiscloud init`) and enables all services.

## Installer (Calamares)

The ISO is a **live image**: it boots a minimal graphical session
(Xorg + openbox, root autologin) and auto-launches the Calamares installer
with THISCLOUD branding. The installer writes to disk (partition via
KPMcore, filesystem copy via unpackfs, GRUB via the bootloader module) and
runs a custom **thiscloud** job module that writes `/etc/thiscloud/config.toml`
and runs `thiscloud init --ip <ip> --role <role>` in the target.

Custom pieces under `iso/calamares/`:

- `branding/thiscloud/` — branding.desc, colors.conf, stylesheet.qss, show.qml, generated PNGs.
- `modules/thiscloudqml/` — QML view module collecting node role/cluster/IP/interface (compiled into Calamares).
- `modules/thiscloud/` — Python job module applying config to the target.
- `settings.conf` — module sequence.
- `scripts/build-calamares.sh` — compiles Calamares 3.3.14 + KPMcore 24.05.2 (absent from EPEL9) into a staging root.
- `scripts/build-live-iso.sh` — assembles the live ISO with livemedia-creator.
- `live/live.ks` — live host kickstart (autologin + Calamares autostart).

Builder requirements (AlmaLinux 9 x86_64): see `install-deps.sh`. The old
Anaconda kickstart (`kickstart/thiscloud.ks`) and `make-product-img.sh`/
`remix-iso.sh` path are replaced by this flow.

## Services on the installed system

| Service                  | Port  | Description                              |
|--------------------------|-------|------------------------------------------|
| `thiscloudd.service`     | 8080  | Rust hypervisor daemon                   |
| `thiscloud-api.service`  | 8081  | Go orchestrator API server               |
| `thiscloud-webui.service`| 3000  | Next.js web UI server                    |
| `nginx.service`          | 80    | Reverse proxy → web UI on port 3000      |
| `etcd.service`           | 2379  | Key-value store for cluster state         |

## Updating a running system

Installed systems pull updates from **GitHub Releases** via the CLI — no need to
reinstall the ISO for a fix or new feature.

```sh
thiscloud update --check      # is a newer release available?
sudo thiscloud update         # download + install + restart services
thiscloud update --version    # print the installed version
```

- Every release branch (`release-v0.2.0`) triggers `.github/workflows/release.yml`,
  which runs tests, builds RPMs + binaries, and attaches them to a GitHub
  Release tagged `v0.2.0` at the branch head. That tag then triggers
  `.github/workflows/iso.yml`, which builds and publishes the ISO.
- Tests (Rust/Go/web-ui) and OpenAPI lint (`.spectral.yaml`) run inside both
  `release.yml` and `iso.yml` — there is no separate per-push/PR check workflow.
- The ISO can also be built manually from the Actions tab
  (`iso.yml` → Run workflow); version is auto-bumped from the latest release.
- `thiscloud update` downloads `manifest.json` first, verifies the sha256 of every
  asset, backs up the current state to `/etc/thiscloud/backup-v<ver>/`, installs,
  restarts services, and records the new version in `/etc/thiscloud/version`.
- On any failure it rolls back binaries, systemd units, and the web UI, and
  attempts a `dnf downgrade` of the RPM packages.
- Override the repo with `THISCLOUD_UPDATE_REPO=owner/repo`; pass a GitHub token
  via `THISCLOUD_UPDATE_TOKEN` to avoid rate limits / reach private repos.

## Dependency matrix

| Component        | Source                              | Install method               |
|------------------|-------------------------------------|------------------------------|
| cloud-hypervisor | GitHub static binary                | staged into repo + /usr/local/bin |
| thiscloud-api    | Built from go-api/ source           | staged into repo + /usr/local/bin |
| web-ui           | Built from web-ui/ (Next.js)        | staged into repo + /usr/share/thiscloud/web-ui |
| OVN / OVS        | CentOS 9 NFV SIG                    | RPM (`openvswitch`, `ovn*`)  |
| DRBD             | ELRepo                              | RPM (`drbd`, `drbd-utils`)   |
| Linstor          | ELRepo / LINBIT                     | RPM (`linstor*`)             |
| etcd             | AlmaLinux AppStream / EPEL          | RPM (`etcd`)                 |
| nginx            | AlmaLinux AppStream                 | RPM + reverse proxy config   |
| Node.js 18       | AlmaLinux AppStream                 | RPM (`@nodejs:18/common`)    |
| qemu-kvm         | AlmaLinux AppStream                 | RPM (VM fallback)            |

## Notes

- `scripts/build-iso.sh` requires `lorax`, `createrepo_c`, `cpio`, `curl`, and `go`.
- The web-ui build requires `node` and `npm` (or `pnpm`). It uses Next.js standalone
  mode so the built output is self-contained — only Node.js is needed on the target.
- The `repo --name="thiscloud-local"` entry in `calamares/live/live.ks` points
  at the host-visible THISCLOUD repo (`file:///data/thiscloud-repo`) that
  `build-live-iso.sh` syncs from `iso/repo` before running livemedia-creator.
- Firewall ports opened: 80 (HTTP/nginx), 8080 (daemon), 8081 (API), 2379-2380 (etcd).
