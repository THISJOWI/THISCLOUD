# THISCLOUD Update Mechanism — `thiscloud update` + GitHub Releases

**Date:** 2026-08-15
**Status:** Approved design
**Goal:** Allow updating a running THISCLOUD system without reinstalling the ISO, following the Proxmox-style "update the running OS" model.

## Problem

Today a THISCLOUD host can only be updated by rebuilding the ISO and reinstalling from scratch. The kickstart does a clean install (partitioning, package install, config generation). There is no path to apply a fix or new feature to an already-deployed host.

## Approach (chosen: Option A)

Distribution via **GitHub Releases**, installed by a new **`thiscloud update`** CLI command.

```
Developer tags release (git tag v0.2.0)
        ↓
GitHub Actions: build RPMs + binaries → upload to GitHub Releases
        ↓
VM: thiscloud update
        ↓
1. Read current version (/etc/thiscloud/version)
2. Query GitHub API (latest release)
3. Compare versions
4. Download assets (RPMs + binaries)
5. Install via dnf/rpm
6. Restart services (thiscloudd, thiscloud-api, thiscloud-webui)
7. Update /etc/thiscloud/version
```

Not chosen:
- **Option B (DNF repo on GitHub Pages):** native `dnf update` experience, but GitHub Pages has a 1GB limit that RPMs + dependency RPMs would blow through quickly.
- **Option C (self-hosted local repo):** more complex; needs a local repo cache to maintain.

## Version tracking

- **Build time:** Rust crates use `env!("CARGO_PKG_VERSION")` (semver from workspace `Cargo.toml`).
- **Installed:** `/etc/thiscloud/version` holds the installed version.
  - Created in kickstart `%post` with `echo "0.1.0" > /etc/thiscloud/version`.
  - Rewritten after each successful update.
- **Format:** strict semver (`0.2.0`). Compare with a semver parser.

## GitHub Release assets

One release per tagged version (`v*`), produced by CI:

```
thiscloudd-<ver>-1.x86_64.rpm         # daemon RPM
thiscloud-<ver>-1.x86_64.rpm          # CLI RPM
thiscloud-api-linux-amd64             # Go API binary
thiscloud-webui-<ver>.tar.gz          # Next.js standalone output
thiscloud-systemd-<ver>.tar.gz        # systemd unit files
manifest.json                         # asset name → sha256, version, release date
```

`manifest.json` is downloaded first; its checksums gate installation (integrity check before touching the system).

## `thiscloud update` subcommand

New file: `thiscloud-cli/src/commands/update.rs`. Registered in `commands/mod.rs` and `main.rs`.

### Commands

```
thiscloud update              # check + install if newer version exists
thiscloud update --check      # check only, no install
thiscloud update --version    # print current installed version
```

### Behavior

- **Root required** for install. If not root: print what's available and exit (no install).
- **Repo config:** default repo URL is the public GitHub repo. Overridable via env `THISCLOUD_UPDATE_REPO` (owner/repo) and `THISCLOUD_UPDATE_TOKEN` (GitHub token for rate limits / private repos).
- **Update steps:**
  1. Read `/etc/thiscloud/version`. If missing, warn and proceed (treat as ancient/unknown).
  2. Fetch latest release metadata from GitHub API (`GET /repos/{owner}/{repo}/releases/latest`).
  3. Compare semver. If installed >= latest, report up-to-date and exit 0.
  4. Download `manifest.json` + all assets into a temp dir.
  5. Verify each asset sha256 against manifest.
  6. **Backup** `/etc/thiscloud/` to `/etc/thiscloud/backup-v{old}/`.
  7. Install:
     - RPMs: `dnf localinstall -y *.rpm` (or `rpm -Uvh` fallback).
     - `thiscloud-api`: replace `/usr/local/bin/thiscloud-api`.
     - web-ui: replace `/usr/share/thiscloud/web-ui/` (preserve `server.js` runtime config if any; web-ui config lives in `/etc/thiscloud/web-ui.env`, untouched).
     - systemd units: replace files in `/etc/systemd/system/`.
  8. `systemctl daemon-reload` then restart `thiscloudd`, `thiscloud-api`, `thiscloud-webui` (only those that exist).
  9. Write new version to `/etc/thiscloud/version`.
- **Rollback:** if install or service restart fails, restore backup of `/etc/thiscloud/`, previous binaries, and previous systemd units; `daemon-reload` + restart; report error with rollback notice.

### Error handling / edge cases

| Case | Handling |
|------|----------|
| Not root | Check-only; instruct to rerun with `sudo` |
| No network | Clear error: cannot reach GitHub API |
| GitHub API rate limit | Retry with backoff; support `THISCLOUD_UPDATE_TOKEN` |
| Corrupt/mismatched download | Manifest sha256 check fails; abort, no system changes |
| User-modified configs | `/etc/thiscloud/*` backed up, never overwritten; install never touches them |
| Service fails after update | Rollback to previous version assets; restart; report |
| Partial update | Manifest checks all assets before any install step |

## CI: GitHub Actions release workflow

New file: `.github/workflows/release.yml`. Trigger: `on: push: tags: ['v*']`.

Steps:
1. Checkout.
2. Install `protoc` (Rust workspace build dep).
3. Cross-compile Rust (`x86_64-unknown-linux-musl`) → `cargo generate-rpm` → RPMs (mirrors `build-iso.sh` steps 1–2).
4. Build Go API (`CGO_ENABLED=0 GOOS=linux GOARCH=amd64`).
5. Build web-ui standalone (`npm install && npm run build`, copy `.next/standalone` + `public` + `package.json`).
6. Bundle systemd units from `iso/systemd/`.
7. Generate `manifest.json` with sha256 of every asset.
8. Create GitHub Release (tag name from `v*`) with all assets attached.

The workflow must be runnable on an **ubuntu runner** (AlmaLinux-only tools are not needed since we only build artifacts, not an ISO). This mirrors the existing `THISCLOUD_BUILD_ONLY=1` split-build path in `build-iso.sh`.

## Files to change

| File | Change |
|------|--------|
| `platform/thiscloud-cli/src/commands/update.rs` | **New** — update logic |
| `platform/thiscloud-cli/src/commands/mod.rs` | Add `pub mod update;` + re-export |
| `platform/thiscloud-cli/src/main.rs` | Add `Update` subcommand + dispatch |
| `platform/thiscloud-cli/Cargo.toml` | Add `semver` dep |
| `platform/Cargo.toml` | Add `semver` to workspace deps |
| `platform/iso/kickstart/thiscloud.ks` | Create `/etc/thiscloud/version` in `%post` |
| `.github/workflows/release.yml` | **New** — release pipeline |
| `platform/iso/README.md` (or root `README.md`) | Document `thiscloud update` usage |

## Testing

- **CLI unit tests:** version compare (semver), manifest parse/checksum logic, repo URL resolution.
- **CLI integration test:** `thiscloud update --check` against a mocked/offline state — must error cleanly with no network.
- **Manual VM test:** boot ISO, tag a `v0.2.0` release with a bumped asset, run `thiscloud update`, verify binaries replaced and version file updated.

## Out of scope

- Option B (GitHub Pages DNF repo) — future work if storage limits allow.
- Signed RPMs / GPG verification — future work; gpgcheck stays off.
- Automatic/scheduled updates — user-initiated only for now.
- Windows/macOS target hosts — x86_64 Linux only, matching the ISO.