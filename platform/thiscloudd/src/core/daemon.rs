use super::{EventBus, Module, ModuleManager};
use crate::compute::http::{app as http_app, ApiState};
use crate::compute::{
    CloudHypervisor, ComputeModule, HypervisorBackend, MemoryVmStore, MockHypervisor,
};
use crate::config::ThisCloudConfig;
use crate::marketplace::http::{app as marketplace_http_app, MarketplaceApiState};
use crate::marketplace::{
    DockerHubBackend, MarketplaceBackend, MarketplaceModule, MemoryMarketplaceStore,
    MockMarketplaceBackend,
};
use crate::network::http::{app as network_http_app, NetworkApiState};
use crate::network::{MemoryNetworkStore, MockNetworkBackend, NetworkBackend, NetworkModule};
use crate::storage::http::{app as storage_http_app, StorageApiState};
use crate::storage::{MemoryStorageStore, MockStorageBackend, StorageBackend, StorageModule};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Daemon {
    config: ThisCloudConfig,
    event_bus: Arc<EventBus>,
    module_manager: Arc<Mutex<ModuleManager>>,
    http_router: axum::Router,
}

impl Daemon {
    pub fn new(config: ThisCloudConfig) -> Self {
        let backend: Box<dyn HypervisorBackend> = match config.compute.backend.as_str() {
            "cloud-hypervisor" => Box::new(CloudHypervisor::new()),
            _ => Box::new(MockHypervisor::new()),
        };
        let compute = Arc::new(tokio::sync::Mutex::new(ComputeModule::new(
            backend,
            Box::new(MemoryVmStore::default()),
        )));

        let network_backend: Box<dyn NetworkBackend> = match config.network.backend.as_str() {
            "ovn" => Box::new(crate::network::OvnNetworkBackend::new()),
            _ => Box::new(MockNetworkBackend::new()),
        };
        let network = Arc::new(tokio::sync::Mutex::new(NetworkModule::new(
            network_backend,
            Box::new(MemoryNetworkStore::default()),
        )));
        let network_router = network_http_app(NetworkApiState::new(network.clone()));

        let storage_backend: Box<dyn StorageBackend> = match config.storage.backend.as_str() {
            "linstor" => Box::new(crate::storage::LinstorBackend::new()),
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

        let http_router = http_app(ApiState::new(compute.clone()))
            .merge(network_router)
            .merge(storage_router)
            .merge(marketplace_router);

        // Apply auth middleware if configured
        let http_router = if config.auth.enabled {
            if let Some(ref secret) = config.auth.jwt_secret {
                crate::auth::middleware::init_secret(secret.clone());
                tracing::info!("Auth enabled (JWT)");
            }
            http_router.layer(axum::middleware::from_fn(crate::auth::middleware::jwt_auth))
        } else {
            tracing::warn!("Auth DISABLED — all endpoints open (dev mode)");
            http_router
        };

        let mut module_manager = ModuleManager::new();
        module_manager.register(Box::new(ComputeModuleProxy));
        module_manager.register(Box::new(NetworkModuleProxy));
        module_manager.register(Box::new(StorageModuleProxy));
        module_manager.register(Box::new(MarketplaceModuleProxy));

        Self {
            config,
            event_bus: Arc::new(EventBus::new()),
            module_manager: Arc::new(Mutex::new(module_manager)),
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

        let mut manager = self.module_manager.lock().await;
        manager.start_all(&self.event_bus).await?;

        // Write PID file for `thiscloud status` (best-effort; `/var/run` is not
        // writable by unprivileged dev users, so failures are logged not fatal).
        match std::fs::write("/var/run/thiscloudd.pid", format!("{}\n", std::process::id())) {
            Ok(()) => tracing::debug!("PID file written to /var/run/thiscloudd.pid"),
            Err(e) => tracing::debug!("Could not write PID file: {e}"),
        }

        tracing::info!("THISCLOUD daemon started successfully");
        Ok(())
    }

    pub async fn serve_http(router: axum::Router, bind: &str) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(bind).await?;
        tracing::info!("THISCLOUD HTTP API listening on {}", bind);
        axum::serve(listener, router).await?;
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
        let http_task = tokio::spawn(async move {
            if let Err(e) = Self::serve_http(router, &bind).await {
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

struct StorageModuleProxy;

#[async_trait::async_trait]
impl Module for StorageModuleProxy {
    fn name(&self) -> &str {
        "storage"
    }

    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Storage module started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Storage module stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}

struct MarketplaceModuleProxy;

#[async_trait::async_trait]
impl Module for MarketplaceModuleProxy {
    fn name(&self) -> &str {
        "marketplace"
    }

    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Marketplace module started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Marketplace module stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}

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

struct ComputeModuleProxy;

#[async_trait::async_trait]
impl Module for ComputeModuleProxy {
    fn name(&self) -> &str {
        "compute"
    }

    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Compute module started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Compute module stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}
