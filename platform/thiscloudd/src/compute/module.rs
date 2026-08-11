use crate::compute::vm::{VmConfig, VmStatus};
use crate::compute::vmstore::VmStore;
use crate::compute::HypervisorBackend;
use crate::core::{Event, EventBus};

pub struct ComputeModule {
    backend: Box<dyn HypervisorBackend>,
    store: Box<dyn VmStore>,
}

impl ComputeModule {
    pub fn new(backend: Box<dyn HypervisorBackend>, store: Box<dyn VmStore>) -> Self {
        Self { backend, store }
    }

    pub async fn create_vm(&mut self, tenant_id: &str, mut vm: VmConfig) -> anyhow::Result<()> {
        vm.tenant_id = tenant_id.to_string();
        self.store.put(tenant_id, &vm).await?;
        tracing::info!("VM created: {} ({}) tenant={}", vm.name, vm.id, tenant_id);
        Ok(())
    }

    pub async fn get_vm(&self, tenant_id: &str, id: &str) -> anyhow::Result<VmConfig> {
        self.store
            .get(tenant_id, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("VM {} not found", id))
    }

    pub async fn list_vms(&self, tenant_id: &str) -> anyhow::Result<Vec<VmConfig>> {
        self.store.list(tenant_id).await
    }

    pub async fn start_vm(&mut self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let mut vm = self.get_vm(tenant_id, id).await?;
        if vm.status == VmStatus::Running {
            return Ok(());
        }
        self.backend.spawn(&vm).await?;
        vm.status = VmStatus::Running;
        self.store.put(tenant_id, &vm).await?;
        tracing::info!("VM started: {}", vm.name);
        Ok(())
    }

    pub async fn stop_vm(&mut self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let mut vm = self.get_vm(tenant_id, id).await?;
        if vm.status == VmStatus::Stopped {
            return Ok(());
        }
        self.backend.stop(&vm).await?;
        vm.status = VmStatus::Stopped;
        self.store.put(tenant_id, &vm).await?;
        tracing::info!("VM stopped: {}", vm.name);
        Ok(())
    }

    pub async fn delete_vm(&mut self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let vm = self.get_vm(tenant_id, id).await?;
        if vm.status == VmStatus::Running {
            self.backend.stop(&vm).await?;
        }
        self.store.delete(tenant_id, id).await?;
        tracing::info!("VM deleted: {}", vm.name);
        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::core::Module for ComputeModule {
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

impl ComputeModule {
    pub fn publish_event(&self, _event_bus: &EventBus, _event: Event) {}
}
