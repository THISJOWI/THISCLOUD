# AGENTS.md

THISCLOUD — self-hosted cloud platform ("Hypervisor OS") for VMs, networks, storage, apps. Monorepo: all five components live under `platform/`. Detailed design reference: `platform/CLAUDE.md` (loaded automatically when working under `platform/`).

## Components

| Path | Stack | Role |
|---|---|---|
| `platform/thiscloudd/` | Rust (axum) | Daemon, HTTP API `:8080`, routes under `/api/v1` |
| `platform/thiscloud-cli/` | Rust (clap) | `thiscloud` CLI → daemon over HTTP |
| `platform/go-api/` | Go | Orchestrator bridge `:8081`, Terraform-provider-shaped `/api/v1/resources` CRUD |
| `platform/web-ui/` | Next.js 14 (App Router) | Dashboard; talks to Go API, **not** the daemon |
| `platform/iso/` | bash/kickstart | AlmaLinux 9 ISO pipeline |

`docs/api/openapi.yaml` is the single API contract (served by daemon, drives web-ui codegen, linted by spectral in CI).

## Commands

Rust workspace root is `platform/` (Cargo.toml; members `thiscloudd`, `thiscloud-cli`). Go and web-ui have their own roots.

```sh
# Rust (from platform/)
cargo test --workspace
cargo test -p thiscloudd test_name     # single test (filter on fn name)
cargo clippy --all-targets -- -D warnings   # CI gate — must pass clean

# Go (from platform/go-api/)
go test ./...
go test ./internal/api/ -run TestName

# Web UI (from platform/web-ui/)
npm test             # node --test tests/api.test.mjs — single file, no framework
npm run lint
npm run dev          # http://localhost:3000
npm run gen:api      # regenerate src/lib/api-types.ts from docs/api/openapi.yaml
```

## Gotchas

- **`protoc` required to build Rust workspace** (`etcd-client`/prost build dep); CI installs `protobuf-compiler` + `etcd-server` on Linux. On macOS: `brew install protobuf etcd` (or `etcd` via `brew install etcd`).
- **etcd tests** (`tests/core/test_etcd*.rs`) spawn an `etcd` binary via `Command::new("etcd")` and skip gracefully if it's not on PATH — a full `cargo test` on a fresh mac may silently skip them.
- **Rust integration test pattern**: `thiscloudd/tests/core.rs` is a thin `#[path]`-include module list. Add new integration tests as `tests/core/test_*.rs` **plus** a `mod` line in `tests/core.rs`, or they won't run.
- **`npm test` is not jest/vitest** — it runs `node --test tests/api.test.mjs`. Don't add framework-specific test code.
- **Run `npm run gen:api` after editing `docs/api/openapi.yaml`**; `src/lib/api-types.ts` is generated and checked in. CI also fails on OpenAPI lint errors (`.spectral.yaml`, severity error).
- **ISO builds only on AlmaLinux 9 x86_64** (self-hosted runner, `build-iso.sh`). macOS can only cross-compile binaries. The ~2.8GB `AlmaLinux-*.iso` at repo root is a gitignored local artifact — never commit it.
- **Web UI never calls the daemon (`:8080`) directly.** Server components call the Go API at `:8081` via `src/lib/api.ts` (reads `process.env.API_URL`); client components must go through `/api/proxy/[...path]`. Never expose `API_URL`/`NEXT_PUBLIC_API_URL` to the client.

## Architecture notes

- Daemon resource modules (compute/network/storage/marketplace) follow one layered pattern set by `src/core/module.rs`: model → backend trait (with in-memory `Mock*Backend` for tests/macOS; real backends shell out and only run inside the ISO) → store (`Memory*Store`/`Etcd*Store`) → `*Module` → axum `http.rs` → mirrored CLI command. New resource types should copy this pattern.
- `go-api` mirrors a Terraform provider: `internal/state` = tfstate-style store (persisted to `THISCLOUD_STATE_FILE`), `internal/backend` = HTTP client to daemon ("apply"), `internal/api` = generic CRUD. It never manages resources directly — the daemon does.

## Env vars

| Variable | Default | Used by |
|---|---|---|
| `THISCLOUD_API_URL` | `http://127.0.0.1:8080` | CLI → daemon; Go API → daemon |
| `THISCLOUD_STATE_FILE` | `./thiscloud.tfstate` | Go API state store |
| `THISCLOUD_API_BIND` | `127.0.0.1:8081` | Go API bind |
| `NEXT_PUBLIC_API_URL` | `http://127.0.0.1:8081` | web UI (client) |
| `API_URL` | `http://127.0.0.1:8081` | web UI (server) |
