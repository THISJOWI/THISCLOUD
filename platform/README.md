# THISCLOUD platform

Monorepo for the THISCLOUD Hypervisor OS components:

| Path            | Stack                    | Role                                        |
|-----------------|--------------------------|---------------------------------------------|
| `thiscloudd/`   | Rust (axum)              | Daemon, HTTP API `:8080` under `/api/v1`    |
| `thiscloud-cli/`| Rust (clap)              | `thiscloud` CLI → daemon over HTTP          |
| `go-api/`       | Go                       | Orchestrator bridge `:8081` (TF-shaped CRUD)|
| `web-ui/`       | Next.js 14               | Dashboard; talks to the Go API              |
| `iso/`          | bash / kickstart         | AlmaLinux 9 ISO build pipeline              |

## Releasing

Releases are cut from `main` as stable `vX.Y.Z` tags (prereleases are cut from
`develop` as `vX.Y.Z-alpha.N`). The `release` GitHub event builds and attaches
the release packages: `thiscloudd` / `thiscloud-cli` RPMs, the Go `thiscloud-api`
binary, the web-ui tarball, systemd units, and `manifest.json` (with SHA-256
checksums).

## Updating an installed system

On an installed THISCLOUD system the CLI checks GitHub for the latest stable
release and installs it:

```sh
sudo thiscloud update          # check + install
thiscloud update --check       # only report availability
thiscloud update --version     # show installed version
```

- `thiscloud update` consumes `releases/latest`, so **prereleases are ignored** —
  only stable `vX.Y.Z` releases are offered.
- The release tag must be valid semver (`v` prefix optional) and the release must
  carry `manifest.json` plus every asset listed in it (checksums are verified
  before anything is touched).
- `THISCLOUD_UPDATE_REPO=owner/repo` overrides the default `THISJOWI/THISCLOUD`;
  `THISCLOUD_UPDATE_TOKEN` authenticates against the GitHub API.
