# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

THISCLOUD — a self-hosted cloud platform ("Hypervisor OS") for managing VMs, networks, storage, and marketplace apps. Five components in this repo:

- `cli/` — Rust CLI (`thiscloud`). Talks to the daemon over HTTP at `:8080` (`/api/v1`).
- `services/daemon/` — Rust daemon. axum HTTP API at `:8080`; owns compute/network/storage/marketplace modules. All routes live under `/api/v1` (single versioned contract, `docs/api/openapi.yaml` served at `/api/v1/openapi.yaml`).
- `api/` — Go orchestrator API at `:8081`. Bridges the web UI to the daemon; exposes Terraform-provider-shaped CRUD over `/api/v1/resources`.
- `web/` — Next.js 14 dashboard. Portal (`/`), Admin (`/admin`), Console (`/console`). Talks to the Go API, not the daemon.
- `os/` — OS build pipeline (kickstart, calamares, systemd units, build scripts).

## Build & test

### Rust workspace (daemon + CLI)

From repo root. Cargo workspace with members `services/daemon`, `cli`.

```sh
cargo build                       # debug
cargo build --release             # release
cargo test                        # all crates
cargo test -p thiscloudd          # daemon only
cargo test -p thiscloudd test_name   # single test (filter works on test fn name)
cargo clippy --all-targets
```

Rust integration tests live in `services/daemon/tests/` and `cli/tests/`. `services/daemon/tests/core.rs` is a thin file that `#[path]`-includes each file under `tests/core/` as a module — add new integration tests there as `tests/core/test_*.rs` plus one `mod` line in `core.rs`.

### Go API

```sh
cd api
go build ./cmd/api-server
go test ./...                    # all
go test ./internal/api/ -run TestName   # single test
```

### Web UI

```sh
cd web
npm install
npm run dev      # http://localhost:3000
npm run build    # production (used by ISO pipeline)
npm test         # node --test tests/api.test.mjs
npm run lint
```

### ISO

The ISO itself can only be built on **AlmaLinux 9 x86_64** (bare metal or VM). macOS can only cross-compile binaries. See `os/README.md`.

```sh
cd os
build/build-iso.sh             # full pipeline: cross-compile → RPM → api → web → repo → ISO
```

## Runtime layout

| Component          | Port | Service            |
|--------------------|------|--------------------|
| daemon             | 8080 | `thiscloudd.service` |
| Go API             | 8081 | `thiscloud-api.service` |
| Web UI             | 3000 | `thiscloud-webui.service` |
| nginx (→ web UI)   | 80   | `nginx.service`    |
| etcd               | 2379 | `etcd.service`     |

## Architecture

### Daemon module pattern (services/daemon)

Every resource type (compute, network, storage, marketplace) follows the identical layered pattern, set by `src/core/module.rs`:

1. **model** — serde structs (e.g. `network/model.rs` `LogicalNetwork`). Module generates UUID v4 ids, enforces name uniqueness.
2. **backend trait** — async trait with `Mock*Backend` (in-memory, used in tests and macOS dev) and a real impl that shells out (e.g. `OvnNetworkBackend` → `ovn-nbctl`, `Linstor*`, `cloud-hypervisor`). Real backends only run inside the ISO.
3. **store** — `Memory*Store` (default) and `Etcd*Store` persisted in etcd.
4. **module** — `*Module` struct wiring backend + store; business logic, validation, errors.
5. **HTTP** — `http.rs` axum router exposing CRUD.
6. **CLI** — `cli/src/commands/*.rs` mirrors the same resource operations over HTTP.

`docs/superpowers/specs/` holds design docs describing this pattern in detail (e.g. the network module design references "Plan 2 (Compute Module) pattern").

### Go API (orchestrator bridge)

`api/` mirrors a **Terraform provider**: `internal/state` is the tfstate-style store (desired state persisted to `THISCLOUD_STATE_FILE`), `internal/backend` is an HTTP client to the daemon (the "apply"), `internal/api` exposes generic `/api/v1/resources[/{type}[/{id}]]` CRUD. It does not manage resources directly — the daemon does.

### Web UI

Next.js App Router. **Never expose `API_URL` to the client**: server components call the Go API directly (`src/lib/api.ts` reads `process.env.API_URL`); client components go through `/api/proxy/[...path]`. Auth uses a session cookie forwarded as `Authorization: Bearer <session>`; mutations add an `X-CSRF-Token` header.

## Environment variables

| Variable | Default | Used by |
|---|---|---|
| `THISCLOUD_API_URL` | `http://127.0.0.1:8080` | CLI → daemon; Go API → daemon |
| `THISCLOUD_STATE_FILE` | `./thiscloud.tfstate` | Go API state store |
| `THISCLOUD_API_BIND` | `127.0.0.1:8081` | Go API bind address |
| `NEXT_PUBLIC_API_URL` | `http://127.0.0.1:8081` | web UI (client) |
| `API_URL` | `http://127.0.0.1:8081` | web UI (server) |

## Packaging (ISO)

RPM metadata lives in each crate's `Cargo.toml` `[package.metadata.generate-rpm]` (via `cargo-generate-rpm`): `thiscloudd` → `/usr/sbin/thiscloudd` + systemd unit, `thiscloud-cli` → `/usr/bin/thiscloud`. The daemon's `before-build` hook runs `os/build/prepare-rpm.sh` to copy cross-compiled binaries. Go API and web UI binaries are staged directly into `os/repo/`.
