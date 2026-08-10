# Plan 3: Network Module (OVN/OVS) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Network Module to `thiscloudd` that manages logical L2 networks (OVN logical switches) with subnet, gateway, VLAN and DNS, exposed via HTTP and the `thiscloud network` CLI.

**Architecture:** Replicates the proven Plan 2 (Compute Module) pattern exactly: serde model → `NetworkBackend` trait (Mock + OVN) → `NetworkStore` trait (Memory + Etcd) → `NetworkModule` (implements core `Module`) → axum HTTP routes → CLI subcommand. The daemon builds and merges the network router into its global HTTP router and registers the module.

**Tech Stack:** Rust 2021, axum 0.7, tokio 1, serde, anyhow, async-trait, uuid v4, ovn-nbctl (ISO only).

## Global Constraints

- Workspace root: `platform/` (git repo). All paths below are relative to it unless absolute.
- Follow existing patterns from `thiscloudd/src/compute/` exactly (model, backend, store, module, http).
- `module_count()` is asserted in existing daemon tests: `test_daemon_registers_compute_module` asserts `module_count() == 1` and `module_names() == vec!["compute"]`. **This test must be UPDATED** when the daemon starts registering network (count 2, names `["compute", "network"]`).
- `uuid` crate v4 is already a dependency of `thiscloudd`.
- Real OVN backend only runs inside the ISO; on macOS/dev the mock backend is used. `OvnNetworkBackend` tests only verify command construction, never execute `ovn-nbctl`.
- No comments in code unless the existing file already uses them for the same purpose (match the pattern).
- Each task ends with a commit.

---

### Task 1: Network model — `LogicalNetwork` + `NetworkStatus`

**Files:**
- Create: `thiscloudd/src/network/network.rs`
- Test: `thiscloudd/tests/core/test_network_model.rs`

**Interfaces:**
- Produces: `LogicalNetwork { id: String, name: String, cidr: String, gateway: String, vlan: Option<u16>, dns: Vec<String>, status: NetworkStatus }`, `NetworkStatus::{Created, Deleted}`, plus a `new(id, name, cidr, gateway) -> Self` constructor (empty vlan/dns, status `Created`).

- [ ] **Step 1: Write the failing test**

`thiscloudd/tests/core/test_network_model.rs`:

```rust
use thiscloudd::network::{LogicalNetwork, NetworkStatus};

#[test]
fn test_network_model_serde_roundtrip() {
    let net = LogicalNetwork::new(
        "net-1".to_string(),
        "web".to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    );
    let json = serde_json::to_string(&net).unwrap();
    let parsed: LogicalNetwork = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, net);
}

#[test]
fn test_network_model_defaults() {
    let net = LogicalNetwork::new(
        "net-1".to_string(),
        "web".to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    );
    assert_eq!(net.status, NetworkStatus::Created);
    assert_eq!(net.vlan, None);
    assert!(net.dns.is_empty());
}

#[test]
fn test_network_status_serde() {
    assert_eq!(
        serde_json::to_string(&NetworkStatus::Created).unwrap(),
        "\"created\""
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test network_model --no-run` or `cargo test -p thiscloudd test_network_model`
Expected: FAIL — module `network` does not exist.

- [ ] **Step 3: Write minimal implementation**

`thiscloudd/src/network/network.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkStatus {
    Created,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalNetwork {
    pub id: String,
    pub name: String,
    pub cidr: String,
    pub gateway: String,
    #[serde(default)]
    pub vlan: Option<u16>,
    #[serde(default)]
    pub dns: Vec<String>,
    #[serde(default)]
    pub status: NetworkStatus,
}

impl LogicalNetwork {
    pub fn new(id: String, name: String, cidr: String, gateway: String) -> Self {
        Self {
            id,
            name,
            cidr,
            gateway,
            vlan: None,
            dns: Vec::new(),
            status: NetworkStatus::Created,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p thiscloudd test_network_model`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add thiscloudd/src/network/network.rs thiscloudd/tests/core/test_network_model.rs
git commit -m "feat(network): LogicalNetwork model with serde"
```

---

### Task 2: Backend trait + `MockNetworkBackend`

**Files:**
- Create: `thiscloudd/src/network/backend.rs`
- Test: `thiscloudd/tests/core/test_network_backend.rs`

**Interfaces:**
- Consumes: `LogicalNetwork` from Task 1.
- Produces: `trait NetworkBackend: Send + Sync { async fn create(&self, &LogicalNetwork) -> Result<()>; async fn delete(&self, &LogicalNetwork) -> Result<()>; async fn exists(&self, id: &str) -> Result<bool>; }`, `MockNetworkBackend::new()`.

- [ ] **Step 1: Write the failing test**

`thiscloudd/tests/core/test_network_backend.rs`:

```rust
use thiscloudd::network::{LogicalNetwork, MockNetworkBackend, NetworkBackend};

fn sample_net(id: &str) -> LogicalNetwork {
    LogicalNetwork::new(
        id.to_string(),
        id.to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    )
}

#[tokio::test]
async fn test_mock_backend_create_and_exists() {
    let backend = MockNetworkBackend::new();
    let net = sample_net("net-1");
    backend.create(&net).await.unwrap();
    assert!(backend.exists("net-1").await.unwrap());
    assert!(!backend.exists("nope").await.unwrap());
}

#[tokio::test]
async fn test_mock_backend_delete() {
    let backend = MockNetworkBackend::new();
    let net = sample_net("net-1");
    backend.create(&net).await.unwrap();
    backend.delete(&net).await.unwrap();
    assert!(!backend.exists("net-1").await.unwrap());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p thiscloudd test_network_backend`
Expected: FAIL — `MockNetworkBackend`/`NetworkBackend` not found.

- [ ] **Step 3: Write minimal implementation**

`thiscloudd/src/network/backend.rs`:

```rust
use crate::network::LogicalNetwork;
use std::collections::HashSet;
use std::sync::Mutex;

#[async_trait::async_trait]
pub trait NetworkBackend: Send + Sync {
    async fn create(&self, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn delete(&self, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn exists(&self, id: &str) -> anyhow::Result<bool>;
}

#[derive(Default)]
pub struct MockNetworkBackend {
    ids: Mutex<HashSet<String>>,
}

impl MockNetworkBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl NetworkBackend for MockNetworkBackend {
    async fn create(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        self.ids.lock().unwrap().insert(net.id.clone());
        Ok(())
    }

    async fn delete(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        self.ids.lock().unwrap().remove(&net.id);
        Ok(())
    }

    async fn exists(&self, id: &str) -> anyhow::Result<bool> {
        Ok(self.ids.lock().unwrap().contains(id))
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p thiscloudd test_network_backend`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add thiscloudd/src/network/backend.rs thiscloudd/tests/core/test_network_backend.rs
git commit -m "feat(network): NetworkBackend trait with mock impl"
```

---

### Task 3: `OvnNetworkBackend` (command construction)

**Files:**
- Modify: `thiscloudd/src/network/backend.rs`
- Test: `thiscloudd/tests/core/test_network_backend.rs`

**Interfaces:**
- Consumes: `NetworkBackend` trait, `LogicalNetwork` (Task 1/2).
- Produces: `OvnNetworkBackend::new()` implementing `NetworkBackend` using `ovn-nbctl`; exposes `fn create_command(&self, net: &LogicalNetwork) -> Vec<String>` and `fn delete_command(&self, net: &LogicalNetwork) -> Vec<String>` for testable command construction.

- [ ] **Step 1: Write the failing test**

Append to `thiscloudd/tests/core/test_network_backend.rs`:

```rust
use thiscloudd::network::OvnNetworkBackend;

#[test]
fn test_ovn_backend_create_command() {
    let backend = OvnNetworkBackend::new();
    let net = sample_net("net-1");
    let cmd = backend.create_command(&net);
    assert_eq!(cmd, vec!["ovn-nbctl", "ls-add", "net-1"]);
}

#[test]
fn test_ovn_backend_delete_command() {
    let backend = OvnNetworkBackend::new();
    let net = sample_net("net-1");
    let cmd = backend.delete_command(&net);
    assert_eq!(cmd, vec!["ovn-nbctl", "ls-del", "net-1"]);
}

#[test]
fn test_ovn_backend_set_subnet_command() {
    let backend = OvnNetworkBackend::new();
    let net = sample_net("net-1");
    let cmd = backend.set_subnet_command(&net);
    assert_eq!(
        cmd,
        vec![
            "ovn-nbctl",
            "set",
            "Logical_Switch",
            "net-1",
            "other_config:subnet=10.0.0.0/24"
        ]
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p thiscloudd test_ovn_backend`
Expected: FAIL — `OvnNetworkBackend` not found.

- [ ] **Step 3: Write minimal implementation**

Append to `thiscloudd/src/network/backend.rs`:

```rust
pub struct OvnNetworkBackend;

impl OvnNetworkBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn create_command(&self, net: &LogicalNetwork) -> Vec<String> {
        vec!["ovn-nbctl".to_string(), "ls-add".to_string(), net.id.clone()]
    }

    pub fn set_subnet_command(&self, net: &LogicalNetwork) -> Vec<String> {
        vec![
            "ovn-nbctl".to_string(),
            "set".to_string(),
            "Logical_Switch".to_string(),
            net.id.clone(),
            format!("other_config:subnet={}", net.cidr),
        ]
    }

    pub fn delete_command(&self, net: &LogicalNetwork) -> Vec<String> {
        vec!["ovn-nbctl".to_string(), "ls-del".to_string(), net.id.clone()]
    }
}

#[async_trait::async_trait]
impl NetworkBackend for OvnNetworkBackend {
    async fn create(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        self.run(&self.create_command(net)).await?;
        self.run(&self.set_subnet_command(net)).await?;
        Ok(())
    }

    async fn delete(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        self.run(&self.delete_command(net)).await?;
        Ok(())
    }

    async fn exists(&self, id: &str) -> anyhow::Result<bool> {
        let output = tokio::process::Command::new("ovn-nbctl")
            .arg("ls-list")
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().any(|l| l.contains(id)))
    }
}

impl OvnNetworkBackend {
    async fn run(&self, cmd: &[String]) -> anyhow::Result<()> {
        let status = tokio::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .status()
            .await?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("ovn-nbctl command failed: {:?}", status)
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p thiscloudd test_ovn_backend`
Expected: PASS (3 tests). The async tests from Task 2 also still pass.

- [ ] **Step 5: Commit**

```bash
git add thiscloudd/src/network/backend.rs thiscloudd/tests/core/test_network_backend.rs
git commit -m "feat(network): OvnNetworkBackend with ovn-nbctl commands"
```

---

### Task 4: Store trait + `MemoryNetworkStore` + `EtcdNetworkStore`

**Files:**
- Create: `thiscloudd/src/network/store.rs`
- Test: `thiscloudd/tests/core/test_network_store.rs`

**Interfaces:**
- Consumes: `LogicalNetwork` (Task 1), `EtcdClient` from `crate::core`.
- Produces: `trait NetworkStore: Send + Sync { put(&self, &LogicalNetwork) -> Result<()>; get(&self, id: &str) -> Result<Option<LogicalNetwork>>; list(&self) -> Result<Vec<LogicalNetwork>>; delete(&self, id: &str) -> Result<()>; }`, `MemoryNetworkStore` (Clone, Default), `EtcdNetworkStore::new(EtcdClient)`.

- [ ] **Step 1: Write the failing test**

`thiscloudd/tests/core/test_network_store.rs`:

```rust
use thiscloudd::network::{LogicalNetwork, MemoryNetworkStore, NetworkStore};

fn sample_net(id: &str) -> LogicalNetwork {
    LogicalNetwork::new(
        id.to_string(),
        id.to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    )
}

#[tokio::test]
async fn test_memory_store_put_get_list_delete() {
    let store = MemoryNetworkStore::default();
    store.put(&sample_net("net-1")).await.unwrap();
    store.put(&sample_net("net-2")).await.unwrap();

    let got = store.get("net-1").await.unwrap().unwrap();
    assert_eq!(got.name, "net-1");

    let all = store.list().await.unwrap();
    assert_eq!(all.len(), 2);

    store.delete("net-1").await.unwrap();
    assert!(store.get("net-1").await.unwrap().is_none());
    assert_eq!(store.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_memory_store_get_missing() {
    let store = MemoryNetworkStore::default();
    assert!(store.get("missing").await.unwrap().is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p thiscloudd test_network_store`
Expected: FAIL — module items not found.

- [ ] **Step 3: Write minimal implementation**

`thiscloudd/src/network/store.rs`:

```rust
use crate::network::LogicalNetwork;
use crate::core::EtcdClient;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait NetworkStore: Send + Sync {
    async fn put(&self, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<LogicalNetwork>>;
    async fn list(&self) -> anyhow::Result<Vec<LogicalNetwork>>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryNetworkStore {
    networks: Arc<Mutex<HashMap<String, LogicalNetwork>>>,
}

#[async_trait::async_trait]
impl NetworkStore for MemoryNetworkStore {
    async fn put(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        self.networks.lock().unwrap().insert(net.id.clone(), net.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<LogicalNetwork>> {
        Ok(self.networks.lock().unwrap().get(id).cloned())
    }

    async fn list(&self) -> anyhow::Result<Vec<LogicalNetwork>> {
        Ok(self.networks.lock().unwrap().values().cloned().collect())
    }

    async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.networks.lock().unwrap().remove(id);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdNetworkStore {
    client: EtcdClient,
}

impl EtcdNetworkStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(id: &str) -> String {
        format!("/thiscloud/networks/{}", id)
    }
}

#[async_trait::async_trait]
impl NetworkStore for EtcdNetworkStore {
    async fn put(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        let json = serde_json::to_string(net)?;
        self.client.put(&Self::key(&net.id), &json).await
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<LogicalNetwork>> {
        match self.client.get(&Self::key(id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<LogicalNetwork>> {
        Err(anyhow::anyhow!(
            "list not supported for EtcdNetworkStore yet; use a prefix range"
        ))
    }

    async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(id)).await
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p thiscloudd test_network_store`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add thiscloudd/src/network/store.rs thiscloudd/tests/core/test_network_store.rs
git commit -m "feat(network): NetworkStore trait with memory and etcd impls"
```

---

### Task 5: `NetworkModule` (CRUD + Module impl)

**Files:**
- Create: `thiscloudd/src/network/module.rs`
- Test: `thiscloudd/tests/core/test_network_module.rs`

**Interfaces:**
- Consumes: `LogicalNetwork`, `NetworkBackend`, `NetworkStore` (Tasks 1–4), `EventBus`/`Event` from `crate::core`.
- Produces: `NetworkModule::new(Box<dyn NetworkBackend>, Box<dyn NetworkStore>)` with `create_network(&mut self, &mut LogicalNetwork) -> Result<()>`, `get_network(&self, id: &str) -> Result<LogicalNetwork>`, `list_networks(&self) -> Result<Vec<LogicalNetwork>>`, `delete_network(&mut self, id: &str) -> Result<()>`, `publish_event(&self, &EventBus, Event)`. Implements `crate::core::Module` with `name() == "network"`.

- [ ] **Step 1: Write the failing test**

`thiscloudd/tests/core/test_network_module.rs`:

```rust
use thiscloudd::network::{LogicalNetwork, MemoryNetworkStore, MockNetworkBackend, NetworkModule};

fn sample_net(id: &str) -> LogicalNetwork {
    LogicalNetwork::new(
        id.to_string(),
        id.to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    )
}

fn module() -> NetworkModule {
    NetworkModule::new(
        Box::new(MockNetworkBackend::new()),
        Box::new(MemoryNetworkStore::default()),
    )
}

#[tokio::test]
async fn test_network_module_create_and_list() {
    let mut m = module();
    m.create_network(&mut sample_net("web")).await.unwrap();
    m.create_network(&mut sample_net("db")).await.unwrap();
    assert_eq!(m.list_networks().await.unwrap().len(), 2);
}

#[tokio::test]
async fn test_network_module_get() {
    let mut m = module();
    m.create_network(&mut sample_net("web")).await.unwrap();
    let net = m.get_network("web").await.unwrap();
    assert_eq!(net.name, "web");
    assert_eq!(net.cidr, "10.0.0.0/24");
}

#[tokio::test]
async fn test_network_module_get_missing_errors() {
    let m = module();
    assert!(m.get_network("nope").await.is_err());
}

#[tokio::test]
async fn test_network_module_duplicate_name_errors() {
    let mut m = module();
    m.create_network(&mut sample_net("web")).await.unwrap();
    let mut dup = sample_net("web");
    let err = m.create_network(&mut dup).await.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn test_network_module_delete() {
    let mut m = module();
    m.create_network(&mut sample_net("web")).await.unwrap();
    m.delete_network("web").await.unwrap();
    assert!(m.list_networks().await.unwrap().is_empty());
}

#[tokio::test]
async fn test_network_module_name() {
    assert_eq!(module().name(), "network");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p thiscloudd test_network_module`
Expected: FAIL — `NetworkModule` not found.

- [ ] **Step 3: Write minimal implementation**

`thiscloudd/src/network/module.rs`:

```rust
use crate::network::{LogicalNetwork, NetworkBackend, NetworkStore};
use crate::core::{Event, EventBus};

pub struct NetworkModule {
    backend: Box<dyn NetworkBackend>,
    store: Box<dyn NetworkStore>,
}

impl NetworkModule {
    pub fn new(backend: Box<dyn NetworkBackend>, store: Box<dyn NetworkStore>) -> Self {
        Self { backend, store }
    }

    pub async fn create_network(&mut self, net: &mut LogicalNetwork) -> anyhow::Result<()> {
        for existing in self.store.list().await? {
            if existing.name == net.name {
                anyhow::bail!("network '{}' already exists", net.name);
            }
        }
        if net.id.is_empty() {
            net.id = uuid::Uuid::new_v4().to_string();
        }
        self.store.put(net).await?;
        self.backend.create(net).await?;
        tracing::info!("Network created: {} ({})", net.name, net.id);
        Ok(())
    }

    pub async fn get_network(&self, id: &str) -> anyhow::Result<LogicalNetwork> {
        self.store
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("network {} not found", id))
    }

    pub async fn list_networks(&self) -> anyhow::Result<Vec<LogicalNetwork>> {
        self.store.list().await
    }

    pub async fn delete_network(&mut self, id: &str) -> anyhow::Result<()> {
        let net = self.get_network(id).await?;
        self.backend.delete(&net).await?;
        self.store.delete(id).await?;
        tracing::info!("Network deleted: {}", net.name);
        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::core::Module for NetworkModule {
    fn name(&self) -> &str {
        "network"
    }

    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Network module started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Network module stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}

impl NetworkModule {
    pub fn publish_event(&self, _event_bus: &EventBus, _event: Event) {
        // Reserved: emits events once HTTP layer is wired.
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p thiscloudd test_network_module`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add thiscloudd/src/network/module.rs thiscloudd/tests/core/test_network_module.rs
git commit -m "feat(network): NetworkModule CRUD with backend + store"
```

---

### Task 6: Network HTTP router (axum)

**Files:**
- Create: `thiscloudd/src/network/http.rs`
- Test: `thiscloudd/tests/core/test_network_http.rs`

**Interfaces:**
- Consumes: `LogicalNetwork`, `NetworkModule` (Tasks 1, 5).
- Produces: `NetworkApiState { module: Arc<Mutex<NetworkModule>> }`, `NetworkApiState::new(Arc<Mutex<NetworkModule>>)`, `pub fn app(state: NetworkApiState) -> axum::Router` with routes `GET /networks`, `POST /networks`, `GET /networks/:id`, `DELETE /networks/:id`.

- [ ] **Step 1: Write the failing test**

`thiscloudd/tests/core/test_network_http.rs`:

```rust
use std::sync::Arc;
use thiscloudd::network::http::{app, NetworkApiState};
use thiscloudd::network::{LogicalNetwork, MemoryNetworkStore, MockNetworkBackend, NetworkModule};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn make_router() -> axum::Router {
    let module = NetworkModule::new(
        Box::new(MockNetworkBackend::new()),
        Box::new(MemoryNetworkStore::default()),
    );
    app(NetworkApiState::new(Arc::new(tokio::sync::Mutex::new(module))))
}

#[tokio::test]
async fn test_network_http_create_and_list() {
    let router = make_router();

    let create_body = r#"{
        "id": "net-1",
        "name": "web",
        "cidr": "10.0.0.0/24",
        "gateway": "10.0.0.1"
    }"#;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/networks")
                .header("content-type", "application/json")
                .body(Body::from(create_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/networks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_network_http_get_and_delete() {
    let router = make_router();

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/networks/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p thiscloudd test_network_http`
Expected: FAIL — `network::http` not found.

- [ ] **Step 3: Write minimal implementation**

`thiscloudd/src/network/http.rs`:

```rust
use crate::network::module::NetworkModule;
use crate::network::LogicalNetwork;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct NetworkApiState {
    pub module: Arc<Mutex<NetworkModule>>,
}

impl NetworkApiState {
    pub fn new(module: Arc<Mutex<NetworkModule>>) -> Self {
        Self { module }
    }
}

pub fn app(state: NetworkApiState) -> Router {
    Router::new()
        .route("/networks", get(list_networks).post(create_network))
        .route("/networks/:id", get(get_network).delete(delete_network))
        .with_state(state)
}

async fn list_networks(
    State(state): State<NetworkApiState>,
) -> Result<Json<Vec<LogicalNetwork>>, NetworkApiError> {
    let module = state.module.lock().await;
    let networks = module.list_networks().await?;
    Ok(Json(networks))
}

async fn create_network(
    State(state): State<NetworkApiState>,
    Json(mut net): Json<LogicalNetwork>,
) -> Result<(StatusCode, Json<LogicalNetwork>), NetworkApiError> {
    let mut module = state.module.lock().await;
    module.create_network(&mut net).await?;
    Ok((StatusCode::CREATED, Json(net)))
}

async fn get_network(
    State(state): State<NetworkApiState>,
    Path(id): Path<String>,
) -> Result<Json<LogicalNetwork>, NetworkApiError> {
    let module = state.module.lock().await;
    let net = module.get_network(&id).await?;
    Ok(Json(net))
}

async fn delete_network(
    State(state): State<NetworkApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, NetworkApiError> {
    let mut module = state.module.lock().await;
    module.delete_network(&id).await?;
    Ok(StatusCode::OK)
}

pub struct NetworkApiError(anyhow::Error);

impl From<anyhow::Error> for NetworkApiError {
    fn from(err: anyhow::Error) -> Self {
        Self(err)
    }
}

impl IntoResponse for NetworkApiError {
    fn into_response(self) -> Response {
        let status = if self.0.to_string().contains("not found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        (
            status,
            Json(serde_json::json!({ "error": self.0.to_string() })),
        )
            .into_response()
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p thiscloudd test_network_http`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add thiscloudd/src/network/http.rs thiscloudd/tests/core/test_network_http.rs
git commit -m "feat(network): axum HTTP router for networks"
```

---

### Task 7: Wire `network` module + config + lib.rs

**Files:**
- Create: `thiscloudd/src/network/mod.rs`
- Modify: `thiscloudd/src/lib.rs`, `thiscloudd/src/config/network.rs`, `thiscloudd/src/config/mod.rs`, `thiscloudd/src/core/daemon.rs`
- Test: `thiscloudd/tests/core/test_network_module.rs` (name test already covers trait), modify `thiscloudd/tests/core/test_daemon_compute.rs`

**Interfaces:**
- Consumes: all Tasks 1–6; `NetworkConfig.backend` new field.
- Produces: `pub mod network` in lib.rs; re-exports from `network/mod.rs`; `NetworkConfig.backend: String` (default `"mock"`); daemon registers network module and merges router.

- [ ] **Step 1: Create `network/mod.rs`, wire `lib.rs`, add `backend` to config**

`thiscloudd/src/network/mod.rs`:

```rust
pub mod backend;
pub mod http;
pub mod module;
pub mod network;
pub mod store;

pub use backend::{MockNetworkBackend, NetworkBackend, OvnNetworkBackend};
pub use http::{app as http_app, NetworkApiState};
pub use module::NetworkModule;
pub use network::{LogicalNetwork, NetworkStatus};
pub use store::{EtcdNetworkStore, MemoryNetworkStore, NetworkStore};
```

Update `thiscloudd/src/lib.rs`:

```rust
pub mod compute;
pub mod config;
pub mod core;
pub mod network;
```

Update `thiscloudd/src/config/network.rs` — add backend field:

```rust
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_management_vlan")]
    pub management_vlan: u16,
    #[serde(default = "default_overlay_type")]
    pub overlay_type: String,
    #[serde(default = "default_backend")]
    pub backend: String,
}

fn default_management_vlan() -> u16 {
    100
}

fn default_overlay_type() -> String {
    "geneve".to_string()
}

fn default_backend() -> String {
    "mock".to_string()
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            management_vlan: default_management_vlan(),
            overlay_type: default_overlay_type(),
            backend: default_backend(),
        }
    }
}

impl NetworkConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
```

`config/mod.rs` needs no change (it already re-exports `NetworkConfig` from `network`).

- [ ] **Step 2: Write the failing tests**

Add to `thiscloudd/tests/core/test_daemon_compute.rs` — first, the aggregated names/count test now expects two modules:

```rust
#[tokio::test]
async fn test_daemon_registers_compute_module() {
    let config = ThisCloudConfig::default();
    let daemon = thiscloudd::core::Daemon::new(config);

    assert_eq!(daemon.module_count(), 2);
    assert_eq!(daemon.module_names(), vec!["compute", "network"]);
}
```

Add the merged-router route test:

```rust
#[tokio::test]
async fn test_daemon_serves_network_and_compute_routes() {
    let config = ThisCloudConfig::default();
    let daemon = thiscloudd::core::Daemon::new(config);

    let app = daemon.http_router();
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/networks")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/vms")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
```

- [ ] **Step 3: Write the daemon wiring**

Update `thiscloudd/src/core/daemon.rs`:

- imports: add

```rust
use crate::network::http::{app as network_http_app, NetworkApiState};
use crate::network::{
    MemoryNetworkStore, MockNetworkBackend, NetworkModule, OvnNetworkBackend,
};
```

- in `new()`, after the compute module is built and before `module_manager` is constructed, build the network module and merge the routers:

```rust
        let network_backend: Box<dyn NetworkBackend> = match config.network.backend.as_str() {
            "ovn" => Box::new(OvnNetworkBackend::new()),
            _ => Box::new(MockNetworkBackend::new()),
        };
        let network = Arc::new(tokio::sync::Mutex::new(NetworkModule::new(
            network_backend,
            Box::new(MemoryNetworkStore::default()),
        )));
        let network_router = network_http_app(NetworkApiState::new(network.clone()));

        let http_router = http_app(ApiState::new(compute.clone())).merge(network_router);
```

(Add `use crate::network::NetworkBackend;` to the imports.)

- register the network module proxy after the compute proxy:

```rust
        module_manager.register(Box::new(NetworkModuleProxy));
```

- add the proxy struct (below `ComputeModuleProxy`):

```rust
struct NetworkModuleProxy;

#[async_trait::async_trait]
impl Module for NetworkModuleProxy {
    fn name(&self) -> &str {
        "network"
    }

    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Network module started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Network module stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}
```

- [ ] **Step 4: Run the full thiscloudd test suite**

Run: `cargo test -p thiscloudd`
Expected: all pass, including `test_daemon_registers_compute_module` (2 modules), `test_daemon_serves_network_and_compute_routes`, and the network module/http/store/backend tests from Tasks 1–6.

- [ ] **Step 5: Commit**

```bash
git add thiscloudd/src/network/mod.rs thiscloudd/src/lib.rs thiscloudd/src/config/network.rs thiscloudd/src/config/mod.rs thiscloudd/src/core/daemon.rs thiscloudd/tests/core/test_daemon_compute.rs
git commit -m "feat(network): wire network module into daemon and config"
```

---

### Task 8: CLI `thiscloud network` subcommand

**Files:**
- Create: `thiscloud-cli/src/commands/network.rs`
- Modify: `thiscloud-cli/src/commands/mod.rs`, `thiscloud-cli/src/main.rs`, `thiscloud-cli/src/commands/init.rs`
- Test: `thiscloud-cli/tests/network_http.rs`

**Interfaces:**
- Consumes: daemon HTTP routes from Task 7.
- Produces: `NetworkCommands` enum (List/Create/Delete) and `run_network_command(command) -> anyhow::Result<()>`.

- [ ] **Step 1: Write the failing test**

`thiscloud-cli/tests/network_http.rs`:

```rust
use std::process::Command;
use std::sync::Arc;

use thiscloudd::network::http::{app, NetworkApiState};
use thiscloudd::network::{MemoryNetworkStore, MockNetworkBackend, NetworkModule};

fn cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_thiscloud")
}

struct ApiServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl ApiServer {
    async fn start() -> Self {
        let module = NetworkModule::new(
            Box::new(MockNetworkBackend::new()),
            Box::new(MemoryNetworkStore::default()),
        );
        let state = NetworkApiState::new(Arc::new(tokio::sync::Mutex::new(module)));
        let router = app(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        Self {
            base_url: format!("http://{}", addr),
            handle,
        }
    }
}

impl Drop for ApiServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_network_create_and_list() {
    let server = ApiServer::start().await;

    let output = Command::new(cli_bin())
        .args([
            "network", "create", "--name", "web", "--cidr", "10.0.0.0/24", "--gateway", "10.0.0.1",
        ])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("web"));

    let output = Command::new(cli_bin())
        .args(["network", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("web"));
    assert!(stdout.contains("10.0.0.0/24"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_network_delete() {
    let server = ApiServer::start().await;

    Command::new(cli_bin())
        .args([
            "network", "create", "--name", "delnet", "--cidr", "10.1.0.0/24",
        ])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    let output = Command::new(cli_bin())
        .args(["network", "delete", "delnet"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new(cli_bin())
        .args(["network", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("delnet"));
}
```

Note: the delete test relies on the CLI passing the value through as an `id`; because `LogicalNetwork` ids are set to the name in the module tests and the CLI create body sets `id` = name, `delete delnet` resolves. If `create_network` generates a UUID when `id` is empty, the CLI create must set `id` explicitly equal to the name (as the compute CLI does: `"id": name`). Keep the create body id equal to name for these tests to work.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p thiscloud-cli test_cli_network`
Expected: FAIL — `network` subcommand not found in CLI.

- [ ] **Step 3: Write minimal implementation**

`thiscloud-cli/src/commands/network.rs`:

```rust
use clap::Subcommand;
use reqwest::Client;
use serde_json::json;

fn api_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

#[derive(Subcommand)]
pub enum NetworkCommands {
    /// List all networks
    List,
    /// Create a new network
    Create {
        /// Network name
        #[arg(long)]
        name: String,
        /// CIDR (e.g. 10.0.0.0/24)
        #[arg(long)]
        cidr: String,
        /// Gateway IP
        #[arg(long)]
        gateway: Option<String>,
        /// VLAN id
        #[arg(long)]
        vlan: Option<u16>,
    },
    /// Delete a network by id
    Delete {
        /// Network id
        id: String,
    },
}

pub async fn run_network_command(command: NetworkCommands) -> anyhow::Result<()> {
    let client = Client::new();
    let base = api_url();

    match command {
        NetworkCommands::List => {
            let resp = client.get(format!("{}/networks", base)).send().await?;
            if !resp.status().is_success() {
                anyhow::bail!("API error: {}", resp.status());
            }
            let networks: Vec<serde_json::Value> = resp.json().await?;
            if networks.is_empty() {
                println!("No networks found");
                return Ok(());
            }
            println!("{:<16} {:<12} {:<18} {:<12}", "ID", "NAME", "CIDR", "GATEWAY");
            for net in networks {
                println!(
                    "{:<16} {:<12} {:<18} {:<12}",
                    net["id"].as_str().unwrap_or(""),
                    net["name"].as_str().unwrap_or(""),
                    net["cidr"].as_str().unwrap_or(""),
                    net["gateway"].as_str().unwrap_or(""),
                );
            }
        }
        NetworkCommands::Create { name, cidr, gateway, vlan } => {
            let mut body = json!({
                "id": name,
                "name": name,
                "cidr": cidr,
                "gateway": gateway.unwrap_or_else(|| "10.0.0.1".to_string()),
            });
            if let Some(v) = vlan {
                body["vlan"] = serde_json::Value::Number(v.into());
            }
            let resp = client
                .post(format!("{}/networks", base))
                .json(&body)
                .send()
                .await?;
            if !resp.status().is_success() {
                let err: serde_json::Value = resp.json().await.unwrap_or(json!({}));
                anyhow::bail!("API error: {}", err["error"].as_str().unwrap_or("unknown"));
            }
            println!("Created network: {}", name);
        }
        NetworkCommands::Delete { id } => {
            let resp = client
                .delete(format!("{}/networks/{}", base, id))
                .send()
                .await?;
            if !resp.status().is_success() {
                let err: serde_json::Value = resp.json().await.unwrap_or(json!({}));
                anyhow::bail!("API error: {}", err["error"].as_str().unwrap_or("unknown"));
            }
            println!("Deleted network: {}", id);
        }
    }

    Ok(())
}
```

Update `thiscloud-cli/src/commands/mod.rs`:

```rust
pub mod init;
pub mod join;
pub mod network;
pub mod status;
pub mod vm;

pub use init::run_init;
pub use join::run_join;
pub use network::run_network_command;
pub use status::run_status;
pub use vm::run_vm_command;
```

Update `thiscloud-cli/src/main.rs` — add the `Network` variant and match arm:

```rust
    /// Manage virtual networks
    Network {
        #[command(subcommand)]
        command: NetworkCommands,
    },
```

and in the match:

```rust
        Commands::Network { command } => commands::run_network_command(command).await,
```

with import `use commands::network::NetworkCommands;`.

Update `thiscloud-cli/src/commands/init.rs` — add a `[network]` section with `backend = "ovn"` to the generated config template (after the `[compute]` section):

```rust
[network]
backend = "ovn"
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p thiscloud-cli test_cli_network`
Expected: PASS (2 tests).

- [ ] **Step 5: Run the entire workspace test suite**

Run: `cargo test --workspace`
Expected: all tests pass.

- [ ] **Step 6: Run clippy + fmt**

Run: `cargo clippy --workspace --all-targets` and `cargo fmt --all -- --check`
Expected: only the two pre-existing `await_holding_lock` warnings in `daemon.rs` (if any); no new warnings.

- [ ] **Step 7: Commit**

```bash
git add thiscloud-cli/src/commands/network.rs thiscloud-cli/src/commands/mod.rs thiscloud-cli/src/main.rs thiscloud-cli/src/commands/init.rs thiscloud-cli/tests/network_http.rs
git commit -m "feat(cli): thiscloud network subcommand"
```

---

### Task 9: Test aggregation + final verification

**Files:**
- Create: `thiscloudd/tests/network.rs`
- Modify: `thiscloudd/tests/core.rs` (if it aggregates via `#[path]`)

**Interfaces:**
- Consumes: test files from Tasks 1–6.

- [ ] **Step 1: Create the aggregation test**

`thiscloudd/tests/network.rs`:

```rust
#[path = "core/test_network_model.rs"]
mod test_network_model;

#[path = "core/test_network_backend.rs"]
mod test_network_backend;

#[path = "core/test_network_store.rs"]
mod test_network_store;

#[path = "core/test_network_module.rs"]
mod test_network_module;

#[path = "core/test_network_http.rs"]
mod test_network_http;
```

Note: if `thiscloudd/tests/core.rs` currently aggregates compute tests, check whether network tests need to be reachable there too. Verify the existing pattern: `tests/compute.rs` aggregates via `#[path]`, so a `tests/network.rs` file with the same pattern is consistent.

- [ ] **Step 2: Run the full workspace suite**

Run: `cargo test --workspace`
Expected: all pass (compute 47 + network new tests).

- [ ] **Step 3: Verify no regressions**

Run: `cargo clippy --workspace --all-targets 2>&1 | grep -E "warning|error" | grep -v "await_holding_lock" | head`
Expected: no output (no new warnings).

- [ ] **Step 4: Update Notion roadmap — Plan 3 → Done**

Use the Notion API: mark Plan 3 ("Plan 3: Network Module (OVN/OVS)") Status → `Done`, update Notas with test count and module summary.

- [ ] **Step 5: Commit**

```bash
git add thiscloudd/tests/network.rs
git commit -m "test(network): aggregate network module tests"
```
