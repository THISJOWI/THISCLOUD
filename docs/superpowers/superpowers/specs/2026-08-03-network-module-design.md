# Plan 3: Network Module (OVN/OVS) — Design

## Goal

Add a Network Module to `thiscloudd` that manages logical L2 networks (OVN logical switches) with subnet, gateway, VLAN and DNS. Replicates the proven Plan 2 (Compute Module) pattern: serde model → backend trait (Mock + real) → store (Memory + Etcd) → module → axum HTTP → CLI.

## Scope

- CRUD of logical L2 networks: `create`, `get`, `list`, `delete`.
- No router L3, no ACLs, no security groups in this phase (YAGNI). Added later if needed.
- No integration with VM start in this phase. VMs continue referencing simple network names; VM↔network wiring happens in a later phase once cloud-hypervisor has full OVN port support.
- Real backend targets OVN/OVS, which only runs inside the ISO. On macOS/dev, the mock backend is used.

## Architecture

Follows the exact structure of `src/compute/`:

### 1. Model — `src/network/network.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogicalNetwork {
    pub id: String,          // UUID v4
    pub name: String,
    pub cidr: String,        // "10.0.0.0/24"
    pub gateway: String,     // "10.0.0.1"
    pub vlan: Option<u16>,   // optional VLAN id
    pub dns: Vec<String>,    // optional DNS servers
    pub status: NetworkStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetworkStatus {
    Created,
    Deleted,
}
```

- Name uniqueness is enforced by the module (error on duplicate name).
- The module generates a UUID v4 for `id` on create if not provided.

### 2. Backend trait — `src/network/backend.rs`

```rust
#[async_trait::async_trait]
pub trait NetworkBackend: Send + Sync {
    async fn create(&self, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn delete(&self, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn exists(&self, id: &str) -> anyhow::Result<bool>;
}
```

Implementations:
- `MockNetworkBackend` — in-memory set of ids; `create`/`delete` mutate the set, `exists` checks membership. Used by tests and macOS dev.
- `OvnNetworkBackend` — shells out to `ovn-nbctl`:
  - create: `ovn-nbctl ls-add <id>` then `ovn-nbctl set Logical_Switch <id> other_config:subnet=<cidr>`
  - delete: `ovn-nbctl ls-del <id>`
  - exists: `ovn-nbctl ls-list` and search for the id.
  - Only runs inside the ISO; not exercised on macOS.

### 3. Store — `src/network/store.rs`

```rust
#[async_trait::async_trait]
pub trait NetworkStore: Send + Sync {
    async fn put(&self, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<LogicalNetwork>>;
    async fn list(&self) -> anyhow::Result<Vec<LogicalNetwork>>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
}
```

Implementations:
- `MemoryNetworkStore` — `Arc<Mutex<HashMap<String, LogicalNetwork>>>`.
- `EtcdNetworkStore` — keys under `/thiscloud/networks/<id>`, JSON serialized. Same caveat as `EtcdVmStore`: `list()` is a placeholder error until a prefix-range is available.

### 4. Module — `src/network/module.rs`

`NetworkModule` implements the core `Module` trait (name `"network"`):

- `create_network(&mut self, net) -> Result<LogicalNetwork>` — enforces unique name, generates id, persists to store, calls backend `create`, returns the stored network.
- `get_network(&self, id) -> Result<LogicalNetwork>` — errors "not found".
- `list_networks(&self) -> Result<Vec<LogicalNetwork>>`.
- `delete_network(&mut self, id) -> Result<()>` — calls backend `delete`, removes from store.

### 5. HTTP — `src/network/http.rs`

axum router (mirrors `compute/http.rs`):

```
GET    /networks        -> list
POST   /networks        -> create (body: LogicalNetwork)
GET    /networks/:id    -> get
DELETE /networks/:id    -> delete
```

- `NetworkApiState { module: Arc<Mutex<NetworkModule>> }`
- `NetworkApiError` mapping "not found" → 404, else 500.

### 6. Config — `src/config/network.rs`

Add to existing `NetworkConfig`:

```toml
[network]
management_vlan = 100
overlay_type = "geneve"
backend = "mock"    # new: "mock" | "ovn"
```

- `backend` defaults to `"mock"`.
- `thiscloud init` writes `backend = "ovn"` in the network section (like compute).

### 7. Daemon — `src/core/daemon.rs`

- Instantiate `NetworkModule` with backend chosen by `config.network.backend`.
- Register it as a real `Module` (improvement over compute's proxy pattern).
- Merge the network router into the daemon's global router.

### 8. CLI — `thiscloud network`

`thiscloud network list` / `create --name ... --cidr ... [--gateway ...] [--vlan ...]` / `delete <id>`.

HTTP calls to `THISCLOUD_API_URL` (default `http://127.0.0.1:8080`), same style as `vm`.

## Error Handling

- Duplicate network name → error with clear message (module-level check).
- Unknown id → `anyhow` error containing "not found" → HTTP 404.
- Backend failures propagate as errors; on macOS with mock backend this never happens.

## Testing

- Unit tests: model serde round-trip, `MockNetworkBackend`, `MemoryNetworkStore`, module CRUD + duplicate-name error.
- HTTP integration tests: spin up real axum server with `MockNetworkBackend`, exercise all four routes (mirrors `vm_http.rs`).
- `OvnNetworkBackend` tests: assert command construction only; never execute `ovn-nbctl`.
- CLI tests: build the daemon's HTTP server in-test and hit it via the CLI code paths.

## Out of Scope (later phases)

- L3 routing, ACLs, security groups, DHCP server.
- VM↔network wiring at `start_vm` time.
- `EtcdNetworkStore::list()` prefix-range support.
