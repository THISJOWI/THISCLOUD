# Clustering: etcd-backed state, self-registering agent, node display

Date: 2026-08-15
Status: Approved (user: "si a todo", proceed to implement)

## Problem

1. **Re-join on every restart.** `daemon.rs` hardcodes `MemoryNodeStore` for the
   node registry and Memory stores for every other module. `Etcd*Store` variants
   exist for all modules but nothing ever connects to etcd at runtime
   (`EtcdManager`/`EtcdConfig`/`with_etcd_stores` are dead code). A master restart
   wipes the registry, so workers must re-`join` each time.
2. **Node display is poor.** `thiscloud node list` shows `cpus_used`/`memory_used`
   as if they were totals, the NAME column is too narrow, and there is no health /
   drain / last-seen signal.
3. **No clustering agent.** `thiscloudd` cannot self-register, report real
   capacity/usage, or survive multi-master setups.

Goal: behavior like Proxmox/OpenStack — state survives restarts, nodes
self-register, multiple masters share one authoritative state.

## Architecture (Enfoque A)

- **State in etcd.** All module stores use their existing `Etcd*Store`.
  Masters form an etcd RAFT cluster at OS level (systemd `etcd.service`).
  `thiscloudd` is only a client — it never manages RAFT membership.
- **Converged nodes.** Any node (master or worker) may host VMs. All daemons
  connect to the same etcd, so state is global and consistent across masters.
- **Agent in thiscloudd.** On boot the daemon self-registers with its persisted
  identity and heartbeats real usage, rotating between masters.
- **Out of scope (follow-up):** distributed remote VM placement/scheduling
  across nodes; etcd CAS/transactions for scheduling and quota.

## Section 1 — etcd wiring

### Config (`[cluster.etcd]`)

```toml
[cluster.etcd]
embedded = false   # true = daemon spawns embedded etcd (single-node/dev)
endpoints = ["http://192.168.1.12:2379", "..."]
```

- `EtcdConfig` gains `endpoints: Vec<String>` (default empty).
- `embedded = true`: daemon spawns embedded etcd via existing `EtcdManager`,
  connect to its endpoint. Default in dev / when nothing else configured.
- `embedded = false`: connect to first reachable external endpoint,
  retry with backoff.

### Startup

- `Daemon::new`/`start` builds one `EtcdClient`:
  - embedded → `EtcdManager::start` + `connect`
  - external → iterate endpoints, connect, backoff retry
- If etcd unreachable: fall back to Memory stores + `warn!` (dev resilience).
- Wire every module store when etcd is available:
  NodeModule, ComputeModule (VmStore), NetworkModule (network/router/dhcp/
  floating-ip via existing `with_etcd_stores`), StorageModule, ImageModule,
  MarketplaceModule, QuotaModule, S3Module, AuditStore.
- Keyspace namespaced by cluster name: `/thiscloud/<cluster>/<type>/<id>`.
  (Etcd*Store key builders currently use `/thiscloud/<type>/<id>`; add the
  cluster segment. Requires key prefix plumbing into the store constructors.)

### Placement/reconciliation

`VmConfig.node` already records the hosting node. Full hypervisor↔etcd
reconciliation on boot is out of scope; the existing HA scan covers
HA-enrolled VMs.

## Section 2 — Self-registering agent

### Identity (`[node]` config)

```toml
[node]
id = "node-7f3a"
role = "worker"     # or "master"
masters = ["http://192.168.1.12:8080", "http://192.168.1.13:8080"]
heartbeat_interval_secs = 10
```

- `NodeConfig.master: String` → `masters: Vec<String>`; keep deserializing the
  singular `master` for backwards compatibility (becomes a 1-element list).

### Boot flow

1. Identity present (`[node] id`) → **self-register upsert** to first live
   master: `POST /nodes` with full Node payload — real `cpus_total` and
   `memory_total_mb` read from `/proc` (`/proc/cpuinfo`, `/proc/meminfo`),
   own `hostname`, `address` (ip:port), role, labels. Idempotent by id.
2. No identity → legacy: master seeds `master-1` when store empty; writes its
   identity so it persists. A node without identity but with a non-empty shared
   store also self-seeds with its `init` role (multi-master).
3. Worker never requires a manual re-join after boot.

### Heartbeat

- Each interval `POST /api/v1/nodes/{id}/heartbeat` with **real** usage:
  `cpus_used`, `memory_used_mb`, `vms` counted from the local compute module.
- Rotate `masters[]` on failure; exponential backoff if all unreachable
  (node goes offline by TTL, revives when a master is reachable again).

### Master endpoint

- `POST /nodes` becomes an idempotent upsert (create-or-update by id).
- Heartbeat route unchanged.

### Service

`thiscloudd.service` already starts on boot; the agent runs in-process, no new
unit.

## Section 3 — Node display (CLI) + config + tests

### `thiscloud node list`

Columns: ID, NAME, ROLE, STATE (colored), CPUS (used/total),
MEMORY (used/total), DRAIN badge, LAST SEEN. Colors: green = online,
red = offline, yellow = draining. Fixed-width alignment.

### `thiscloud node show`

Show total capacity, humanized `last_seen`, drain state.

### OpenAPI

- Update `docs/api/openapi.yaml`: `POST /nodes` upsert semantics, `[node]`
  `masters` list. Run `npm run gen:api` after.

### Tests

- Config: `[cluster.etcd]` embedded/endpoints parsing; `[node] masters`
  (and singular `master` compat).
- Agent: self-registration idempotent (mock master server), master rotation
  on failure, heartbeat reports real usage.
- etcd store: key namespace includes cluster name.
- CLI: table output snapshot for `node list`.

## Error handling

- etcd down at startup → Memory fallback + `warn!`; embedded failure → error.
- All masters unreachable → agent keeps retrying with backoff; state derived
  from TTL (offline).
- Registration is best-effort at boot; heartbeat loop continues regardless.

## Testing strategy

Rust integration tests in `thiscloudd/tests/core/test_*.rs` (registered in
`tests/core.rs`); CLI tests in `thiscloud-cli/tests/cli.rs` with a mock HTTP
server. `cargo test --workspace` + `cargo clippy --all-targets -D warnings`
must pass.

## Follow-ups (not in this iteration)

- Distributed VM placement (schedule onto least-loaded node via remote API).
- etcd CAS/transactions for quota + scheduling races.
- Full hypervisor↔etcd reconciliation on boot.