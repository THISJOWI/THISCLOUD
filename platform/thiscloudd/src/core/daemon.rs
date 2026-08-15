use super::{EventBus, Module, ModuleManager};
use crate::audit::middleware::{audit_middleware, AuditState};
use crate::audit::MemoryAuditStore;
use crate::auth::login::{self, LoginState};
use crate::compute::http::{app as http_app, ApiState};
use crate::compute::{
    CloudHypervisor, ComputeModule, HypervisorBackend, MemoryVmStore, MockHypervisor,
};
use crate::config::ThisCloudConfig;
use crate::image::http::{app as image_http_app, ImageApiState};
use crate::image::{
    ImageBackend, ImageModule, LocalImageBackend, MemoryImageStore, MockImageBackend,
};
use crate::marketplace::http::{app as marketplace_http_app, MarketplaceApiState};
use crate::marketplace::{
    DockerHubBackend, MarketplaceBackend, MarketplaceModule, MemoryMarketplaceStore,
    MockMarketplaceBackend,
};
use crate::metrics::http::{app as metrics_http_app, MetricsApiState};
use crate::metrics::MetricRegistry;
use crate::network::http::{app as network_http_app, NetworkApiState};
use crate::network::{MemoryNetworkStore, MockNetworkBackend, NetworkBackend, NetworkModule};
use crate::node::http::{app as node_http_app, NodeApiState};
use crate::node::model::NodeHeartbeat;
use crate::node::{NodeModule, SelfHeartbeat};
use crate::quota::http::{app as quota_http_app, QuotaApiState};
use crate::quota::QuotaModule;
use crate::s3::http::{app as s3_http_app, S3ApiState};
use crate::s3::{MemoryS3Store, MockS3Backend, RadosgwBackend, S3Backend, S3Module};
use crate::storage::http::{app as storage_http_app, StorageApiState};
use crate::storage::{MemoryStorageStore, MockStorageBackend, StorageBackend, StorageModule};
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Daemon {
    config: ThisCloudConfig,
    event_bus: Arc<EventBus>,
    module_manager: Arc<Mutex<ModuleManager>>,
    node_module: Arc<Mutex<NodeModule>>,
    compute_module: Arc<Mutex<ComputeModule>>,
    ha_interval_secs: u64,
    node_heartbeat_interval_secs: u64,
    http_router: axum::Router,
}

/// Generate a random hex secret for JWT when none is configured.
fn random_secret() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:032x}", t)
}

impl Daemon {
    pub fn new(config: ThisCloudConfig) -> Self {
        // Shared per-tenant quotas (T0.5): wired into compute/network modules
        // for enforcement before resource creation.
        let quota_module = Arc::new(tokio::sync::Mutex::new(QuotaModule::with_memory_store()));
        let quota_router = quota_http_app(QuotaApiState::new(quota_module.clone()));

        let backend: Box<dyn HypervisorBackend> = match config.compute.backend.as_str() {
            "cloud-hypervisor" => Box::new(CloudHypervisor::new()),
            _ => Box::new(MockHypervisor::new()),
        };
        let nodes = Arc::new(tokio::sync::Mutex::new(NodeModule::with_memory_store()));
        let image_backend: Box<dyn ImageBackend> = match config.image.backend.as_str() {
            "local" => Box::new(LocalImageBackend::new(config.image.images_dir.clone())),
            _ => Box::new(MockImageBackend::default()),
        };
        let image = Arc::new(tokio::sync::Mutex::new(ImageModule::new(
            image_backend,
            Box::new(MemoryImageStore::default()),
        )));
        let image_router = image_http_app(ImageApiState::new(image.clone()));
        let compute = Arc::new(tokio::sync::Mutex::new(
            ComputeModule::new(backend, Box::new(MemoryVmStore::default()))
                .with_quota(quota_module.clone())
                .with_nodes(nodes.clone())
                .with_images(image.clone())
                .with_ha(config.ha.enabled, config.ha.quorum),
        ));

        let network_backend: Box<dyn NetworkBackend> = match config.network.backend.as_str() {
            "ovn" => Box::new(crate::network::OvnNetworkBackend::new()),
            _ => Box::new(MockNetworkBackend::new()),
        };
        let network = Arc::new(tokio::sync::Mutex::new(
            NetworkModule::new(network_backend, Box::new(MemoryNetworkStore::default()))
                .with_quota(quota_module.clone()),
        ));
        let network_router = network_http_app(NetworkApiState::new(network.clone()));

        let storage_backend: Box<dyn StorageBackend> = match config.storage.backend.as_str() {
            "linstor" => Box::new(crate::storage::LinstorBackend::new()),
            "ceph" => Box::new(crate::storage::CephBackend::new()),
            _ => Box::new(MockStorageBackend::new()),
        };
        let storage = Arc::new(tokio::sync::Mutex::new(StorageModule::new(
            storage_backend,
            Box::new(MemoryStorageStore::default()),
        )));
        let storage_router = storage_http_app(StorageApiState::new(storage.clone()));

        let marketplace_backend: Box<dyn MarketplaceBackend> =
            match config.marketplace.backend.as_str() {
                "docker" => Box::new(DockerHubBackend::new()),
                _ => Box::new(MockMarketplaceBackend::default()),
            };
        let marketplace = Arc::new(tokio::sync::Mutex::new(MarketplaceModule::new(
            marketplace_backend,
            Box::new(MemoryMarketplaceStore::default()),
        )));
        let marketplace_router =
            marketplace_http_app(MarketplaceApiState::new(marketplace.clone()));

        let audit_store = Arc::new(tokio::sync::Mutex::new(Box::new(MemoryAuditStore::new())
            as Box<dyn crate::audit::store::AuditStore>));
        let audit_state = AuditState { store: audit_store };

        let s3_backend: Box<dyn S3Backend> = match config.s3.backend.as_str() {
            "radosgw" => Box::new(RadosgwBackend::new(config.s3.gateway_host.clone())),
            _ => Box::new(MockS3Backend::new()),
        };
        let s3 = Arc::new(tokio::sync::Mutex::new(S3Module::new(
            s3_backend,
            Box::new(MemoryS3Store::default()),
        )));
        let s3_router = s3_http_app(S3ApiState::new(s3.clone()));

        let metric_registry = Arc::new(MetricRegistry::new());
        let metrics_router = metrics_http_app(MetricsApiState::new(metric_registry.clone()));

        let node_router = node_http_app(NodeApiState::new(nodes.clone()));

        let resource_router = http_app(ApiState::new(compute.clone()))
            .merge(network_router)
            .merge(storage_router)
            .merge(marketplace_router)
            .merge(image_router)
            .merge(node_router)
            .merge(quota_router)
            .merge(s3_router)
            .merge(metrics_router);

        // Audit middleware: records mutations to the shared audit store.
        let resource_router = resource_router.layer(
            axum::middleware::from_fn_with_state(audit_state.clone(), audit_middleware),
        );

        // Public metadata routes (no auth): health + the OpenAPI contract
        // itself, served under the versioned prefix.
        let metadata_router = Router::new()
            .route("/api/v1/healthz", get(healthz))
            .route("/api/v1/openapi.yaml", get(openapi_spec));

        // Build the full router. Everything lives under /api/v1 — this is the
        // single versioned contract (T0.1); unversioned paths now 404.
        // THISCLOUD_AUTH_DISABLED=1 forces auth off regardless of config (dev mode).
        let auth_enabled = {
            let env_disabled = std::env::var("THISCLOUD_AUTH_DISABLED")
                .map(|v| v == "1" || v == "true")
                .unwrap_or(false);
            config.auth.enabled && !env_disabled
        };
        let http_router = if auth_enabled {
            let secret = config
                .auth
                .jwt_secret
                .clone()
                .unwrap_or_else(|| {
                    tracing::warn!("No jwt_secret configured — generating random secret (tokens will not survive restart)");
                    random_secret()
                });
            login::ensure_secret(&secret);
            tracing::info!("Auth enabled (JWT)");

            let login_state = LoginState::new(secret, config.auth.jwt_ttl_secs);
            let login_router = login::router(login_state);

            // Protected surface: resource routes nested under /api/v1, JWT-gated.
            // Login stays public but also lives under /api/v1.
            let protected = Router::new()
                .nest("/api/v1", resource_router)
                .layer(axum::middleware::from_fn(
                    crate::auth::middleware::jwt_auth,
                ));

            metadata_router
                .merge(Router::new().nest("/api/v1", login_router))
                .merge(protected)
        } else {
            tracing::warn!("Auth DISABLED — all endpoints open (dev mode)");
            metadata_router.merge(Router::new().nest("/api/v1", resource_router))
        };

        let mut module_manager = ModuleManager::new();
        module_manager.register(Box::new(ComputeModuleProxy));
        module_manager.register(Box::new(NetworkModuleProxy));
        module_manager.register(Box::new(StorageModuleProxy));
        module_manager.register(Box::new(MarketplaceModuleProxy));
        module_manager.register(Box::new(NodeModuleProxy));
        module_manager.register(Box::new(ImageModuleProxy));
        module_manager.register(Box::new(S3ModuleProxy));
        module_manager.register(Box::new(MetricsModuleProxy));

        let ha_interval_secs = config.ha.scan_interval_secs.max(1);
        let node_heartbeat_interval_secs = config.node.heartbeat_interval_secs.max(1);

        Self {
            config,
            event_bus: Arc::new(EventBus::new()),
            module_manager: Arc::new(Mutex::new(module_manager)),
            node_module: nodes,
            compute_module: compute,
            ha_interval_secs,
            node_heartbeat_interval_secs,
            http_router,
        }
    }

    pub async fn register_module(&mut self, module: Box<dyn Module>) {
        self.module_manager.lock().await.register(module);
    }

    pub async fn module_count(&self) -> usize {
        self.module_manager.lock().await.module_names().len()
    }

    pub async fn module_names(&self) -> Vec<String> {
        self.module_manager
            .lock()
            .await
            .module_names()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn http_router(&self) -> axum::Router {
        self.http_router.clone()
    }

    pub fn http_bind(&self) -> &str {
        &self.config.compute.http_bind
    }

    pub fn http_port(&self) -> u16 {
        self.config.compute.http_port
    }

    pub fn cluster_name(&self) -> &str {
        &self.config.cluster.name
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        tracing::info!("Starting THISCLOUD daemon");
        tracing::info!("Cluster name: {}", self.config.cluster.name);

        // T1.3: self-register the local master node when no nodes are known so
        // the best-fit scheduler has a candidate in single-node/dev mode.
        {
            let mut nodes = self.node_module.lock().await;
            if nodes.is_empty().await? {
                let master = nodes.seed_local_master().await?;
                tracing::info!("Self-registered master node: {} ({})", master.name, master.id);
            }
        }

        // Self-heartbeat loop: keeps this daemon's node entry online. A worker
        // (config `[node]` with `id` + `master`) POSTs heartbeats to the master;
        // a master/single node refreshes its own store in place.
        {
            let node_module = self.node_module.clone();
            let self_id = self.config.node.id.clone().unwrap_or_else(|| "master-1".to_string());
            let master = self.config.node.master.clone();
            let interval = self.node_heartbeat_interval_secs;
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval));
                let heartbeat = SelfHeartbeat::new(self_id, master.clone());
                loop {
                    ticker.tick().await;
                    let usage = NodeHeartbeat {
                        cpus_used: 0,
                        memory_used_mb: 0,
                        vms: 0,
                    };
                    // Only touch the local store on the local path; the remote
                    // path must not hold the lock across the HTTP call.
                    let mut guard = if master.is_some() {
                        None
                    } else {
                        Some(node_module.lock().await)
                    };
                    if let Err(e) = heartbeat.beat(guard.as_deref_mut(), usage).await {
                        tracing::warn!("self heartbeat failed: {:#}", e);
                    }
                }
            });
        }

        let mut manager = self.module_manager.lock().await;
        manager.start_all(&self.event_bus).await?;

        match std::fs::write("/var/run/thiscloudd.pid", format!("{}\n", std::process::id())) {
            Ok(()) => tracing::debug!("PID file written to /var/run/thiscloudd.pid"),
            Err(e) => tracing::debug!("Could not write PID file: {e}"),
        }

        tracing::info!("THISCLOUD daemon started successfully");

        // T1.4: periodic HA failover scan — relocates HA-enrolled running VMs
        // off nodes that stopped heartbeating (subject to quorum).
        {
            let compute_handle = self.compute_module.clone();
            let interval = self.ha_interval_secs;
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval));
                loop {
                    ticker.tick().await;
                    let mut compute = compute_handle.lock().await;
                    match compute.failover_scan().await {
                        Ok(moved) if !moved.is_empty() => tracing::info!(
                            "HA failover moved {} VM(s): {:?}",
                            moved.len(),
                            moved
                        ),
                        Ok(_) => {}
                        Err(e) => tracing::error!("HA failover scan error: {:#}", e),
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn serve_http(router: axum::Router, bind: &str) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(bind).await?;
        tracing::info!("THISCLOUD HTTP API listening on {}", bind);
        axum::serve(listener, router).await?;
        Ok(())
    }

    /// Serve with TLS when cert/key are configured.
    pub async fn serve_https(
        router: axum::Router,
        bind: &str,
        cert_path: &str,
        key_path: &str,
    ) -> anyhow::Result<()> {
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
            cert_path,
            key_path,
        )
        .await?;

        let addr: std::net::SocketAddr = bind.parse()?;
        tracing::info!("THISCLOUD HTTPS API listening on {}", bind);
        axum_server::bind_rustls(addr, tls_config)
            .serve(router.into_make_service())
            .await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        let mut manager = self.module_manager.lock().await;
        manager.stop_all().await?;
        let _ = std::fs::remove_file("/var/run/thiscloudd.pid");
        tracing::info!("THISCLOUD daemon stopped");
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.start().await?;

        let router = self.http_router();
        let bind = format!("{}:{}", self.http_bind(), self.http_port());
        let tls = self.config.auth.tls.clone();
        let http_task = tokio::spawn(async move {
            let result = if tls.enabled {
                match (&tls.cert_path, &tls.key_path) {
                    (Some(cert), Some(key)) => {
                        Self::serve_https(router, &bind, cert, key).await
                    }
                    _ => {
                        tracing::error!("TLS enabled but cert_path/key_path not configured");
                        Ok(())
                    }
                }
            } else {
                Self::serve_http(router, &bind).await
            };
            if let Err(e) = result {
                tracing::error!("HTTP server error: {:#}", e);
            }
        });
        tracing::info!("Waiting for shutdown signal (Ctrl+C)");
        tokio::signal::ctrl_c().await?;

        self.stop().await?;
        http_task.abort();
        Ok(())
    }
}

/// Health check (public): confirms the daemon HTTP surface is up.
async fn healthz() -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({ "status": "ok" }))
}

/// Serves the OpenAPI contract itself, embedded at build time.
///
/// The spec at docs/api/openapi.yaml is the single source of truth (T0.1).
async fn openapi_spec() -> impl axum::response::IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/yaml")],
        include_str!("../../../../docs/api/openapi.yaml"),
    )
}

struct StorageModuleProxy;

#[async_trait::async_trait]
impl Module for StorageModuleProxy {
    fn name(&self) -> &str { "storage" }
    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Storage module started"); Ok(())
    }
    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Storage module stopped"); Ok(())
    }
    fn is_running(&self) -> bool { true }
}

struct MarketplaceModuleProxy;

#[async_trait::async_trait]
impl Module for MarketplaceModuleProxy {
    fn name(&self) -> &str { "marketplace" }
    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Marketplace module started"); Ok(())
    }
    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Marketplace module stopped"); Ok(())
    }
    fn is_running(&self) -> bool { true }
}

struct NetworkModuleProxy;

#[async_trait::async_trait]
impl Module for NetworkModuleProxy {
    fn name(&self) -> &str { "network" }
    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Network module started"); Ok(())
    }
    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Network module stopped"); Ok(())
    }
    fn is_running(&self) -> bool { true }
}

struct ImageModuleProxy;

#[async_trait::async_trait]
impl Module for ImageModuleProxy {
    fn name(&self) -> &str { "image" }
    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Image module started"); Ok(())
    }
    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Image module stopped"); Ok(())
    }
    fn is_running(&self) -> bool { true }
}

struct ComputeModuleProxy;

#[async_trait::async_trait]
impl Module for ComputeModuleProxy {
    fn name(&self) -> &str { "compute" }
    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Compute module started"); Ok(())
    }
    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Compute module stopped"); Ok(())
    }
    fn is_running(&self) -> bool { true }
}

struct NodeModuleProxy;

#[async_trait::async_trait]
impl Module for NodeModuleProxy {
    fn name(&self) -> &str { "node" }
    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Node module started"); Ok(())
    }
    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Node module stopped"); Ok(())
    }
    fn is_running(&self) -> bool { true }
}

struct S3ModuleProxy;

#[async_trait::async_trait]
impl Module for S3ModuleProxy {
    fn name(&self) -> &str { "s3" }
    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("S3 module started"); Ok(())
    }
    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("S3 module stopped"); Ok(())
    }
    fn is_running(&self) -> bool { true }
}

struct MetricsModuleProxy;

#[async_trait::async_trait]
impl Module for MetricsModuleProxy {
    fn name(&self) -> &str { "metrics" }
    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Metrics module started"); Ok(())
    }
    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Metrics module stopped"); Ok(())
    }
    fn is_running(&self) -> bool { true }
}
