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
  kickstart/thiscloud.ks       AlmaLinux 9 kickstart (packages, partitions, %post)
  scripts/cross-compile.sh     thiscloudd/thiscloud → x86_64-unknown-linux-gnu
  scripts/prepare-rpm.sh       cargo-generate-rpm before-build hook (copies binaries)
  scripts/build-iso.sh         full pipeline: cross → RPM → go-api → web-ui → repo → ISO
  scripts/fetch-deps.sh        stage cloud-hypervisor + dependency RPMs + systemd units
  scripts/make-repo.sh         build local RPM repo metadata
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
   `createrepo_c iso/repo/thiscloud` (repo referenced from the kickstart via
   `file:///run/install/repo/thiscloud`).

7. **ISO build** — `scripts/build-iso.sh`
   Uses `livemedia-creator --make-iso` with the kickstart (or `mkksiso` to inject
   the kickstart into the base installer ISO). Produces `ThisCloud-0.1.0-x86_64.iso`.

8. **Test in a VM** — boot the ISO in qemu/libvirt; kickstart auto-partitions,
   installs all packages, runs `thiscloud init`, and enables all services.

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
- Fast checks (Rust/Go/web-ui) run automatically on every push to `main`
  and on PRs (`.github/workflows/checks.yml` → reusable `tests.yml`).
- OpenAPI lint (`.spectral.yaml`) runs in `release.yml` and `iso.yml`, not on
  every push/PR.
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
- The `repo --name="thiscloud"` entry in the kickstart points at the repo
  directory that livemedia-creator copies into the ISO install tree.
- Firewall ports opened: 80 (HTTP/nginx), 8080 (daemon), 8081 (API), 2379-2380 (etcd).
