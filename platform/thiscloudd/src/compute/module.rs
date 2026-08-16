use crate::compute::vm::{ConsoleInfo, DiskConfig, Snapshot, VmConfig, VmStatus};
use crate::compute::vmstore::VmStore;
use crate::compute::HypervisorBackend;
use crate::core::{Event, EventBus};
use crate::image::ImageModule;
use crate::node::{model::NodeState, NodeModule};
use crate::quota::model::ResourceDelta;
use crate::quota::QuotaModule;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct ComputeModule {
    backend: Box<dyn HypervisorBackend>,
    store: Box<dyn VmStore>,
    quota: Option<Arc<Mutex<QuotaModule>>>,
    nodes: Option<Arc<Mutex<NodeModule>>>,
    images: Option<Arc<Mutex<ImageModule>>>,
    ha_enabled: bool,
    ha_quorum: u32,
}

impl ComputeModule {
    pub fn new(backend: Box<dyn HypervisorBackend>, store: Box<dyn VmStore>) -> Self {
        Self {
            backend,
            store,
            quota: None,
            nodes: None,
            images: None,
            ha_enabled: false,
            ha_quorum: 0,
        }
    }

    /// Enable automatic HA failover (T1.4): `quorum` is the minimum number of
    /// online nodes required before a failed node's VMs are migrated away.
    pub fn with_ha(mut self, enabled: bool, quorum: u32) -> Self {
        self.ha_enabled = enabled;
        self.ha_quorum = quorum;
        self
    }

    /// Enable quota enforcement for this module (T0.5).
    pub fn with_quota(mut self, quota: Arc<Mutex<QuotaModule>>) -> Self {
        self.quota = Some(quota);
        self
    }

    /// Enable scheduler placement for this module (T1.3 multi-node).
    pub fn with_nodes(mut self, nodes: Arc<Mutex<NodeModule>>) -> Self {
        self.nodes = Some(nodes);
        self
    }

    /// Enable image resolution for this module (T1.2 image registry).
    pub fn with_images(mut self, images: Arc<Mutex<ImageModule>>) -> Self {
        self.images = Some(images);
        self
    }

    /// Resolve the boot image: translate an image name/id into a disk path
    /// when the caller did not supply one explicitly.
    async fn resolve_image(&self, tenant_id: &str, vm: &mut VmConfig) -> anyhow::Result<()> {
        if vm.image.is_empty() {
            return Ok(());
        }
        let Some(images) = self.images.clone() else {
            anyhow::bail!(
                "vm specifies image '{}' but the image registry is not available",
                vm.image
            );
        };
        let images = images.lock().await;
        let image = match images.get(tenant_id, &vm.image).await {
            Ok(i) => i,
            Err(_) => images
                .get_by_name(tenant_id, &vm.image)
                .await?
                .ok_or_else(|| anyhow::anyhow!("image '{}' not found", vm.image))?,
        };
        if vm.disk_path.is_empty() && image.format != crate::image::ImageFormat::CloudInit {
            vm.disk_path = format!(
                "/var/lib/thiscloud/images/{}.{}",
                image.id,
                match image.format {
                    crate::image::ImageFormat::Qcow2 => "qcow2",
                    crate::image::ImageFormat::Iso => "iso",
                    crate::image::ImageFormat::Raw => "img",
                    crate::image::ImageFormat::CloudInit => "cfg",
                }
            );
        }
        Ok(())
    }

    /// Resolve the placement node for `vm`: honour an explicit `node`, otherwise
    /// let the best-fit scheduler pick one. If scheduling is not wired up, a VM
    /// without a node is left unbound (single-node dev mode).
    async fn place_on_node(&self, vm: &mut VmConfig) -> anyhow::Result<()> {
        let Some(nodes) = self.nodes.clone() else {
            return Ok(());
        };
        let mut nodes = nodes.lock().await;
        if vm.node.is_empty() {
            let id = nodes
                .best_fit(vm.cpus, vm.memory_mb, &vm.affinity, &vm.anti_affinity)
                .await?;
            vm.node = id;
        } else {
            // Explicit node: accept a node name or id, validate online + capacity, then reserve.
            let node_id = nodes.resolve_id(&vm.node).await?;
            nodes.reserve(&node_id, vm.cpus, vm.memory_mb).await?;
            vm.node = node_id;
        }
        Ok(())
    }

    async fn release_from_node(&self, vm: &VmConfig) -> anyhow::Result<()> {
        if let Some(nodes) = self.nodes.clone() {
            if !vm.node.is_empty() {
                nodes
                    .lock()
                    .await
                    .release(&vm.node, vm.cpus, vm.memory_mb)
                    .await?;
            }
        }
        Ok(())
    }

    /// Enforce tenant quotas (cpus, memory, vm count) before creating a VM.
    async fn enforce_quota(&self, tenant_id: &str, vm: &VmConfig) -> anyhow::Result<()> {
        if let Some(quota) = &self.quota {
            let existing = self.store.list(tenant_id).await?;
            let delta = ResourceDelta {
                cpus: existing.iter().map(|v| v.cpus).sum::<u32>() + vm.cpus,
                memory_mb: existing.iter().map(|v| v.memory_mb).sum::<u32>() + vm.memory_mb,
                vms: existing.len() as u32 + 1,
                storage_gb: 0,
                networks: 0,
            };
            quota.lock().await.check(tenant_id, &delta).await?;
        }
        Ok(())
    }

    pub async fn create_vm(&mut self, tenant_id: &str, mut vm: VmConfig) -> anyhow::Result<()> {
        vm.tenant_id = tenant_id.to_string();
        // Guarantee a stable id in the module layer (not just http.rs) so any
        // caller — CLI, web UI, tests, internal flows — stores an addressable
        // VM. Without it, deletes target an empty id.
        if vm.id.is_empty() {
            vm.id = Uuid::new_v4().to_string();
        }
        self.enforce_quota(tenant_id, &vm).await?;
        self.place_on_node(&mut vm).await?;
        self.resolve_image(tenant_id, &mut vm).await?;
        self.store.put(tenant_id, &vm).await?;
        tracing::info!(
            "VM created: {} ({}) tenant={} node={}",
            vm.name,
            vm.id,
            tenant_id,
            vm.node
        );
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
        self.release_from_node(&vm).await?;
        tracing::info!("VM deleted: {}", vm.name);
        Ok(())
    }

    // ── T1.1: full lifecycle ──────────────────────────────

    /// Snapshot a VM; persists the snapshot descriptor on the config.
    pub async fn snapshot_vm(
        &mut self,
        tenant_id: &str,
        id: &str,
        name: &str,
    ) -> anyhow::Result<Snapshot> {
        let mut vm = self.get_vm(tenant_id, id).await?;
        let snap = self.backend.snapshot(&vm, name).await?;
        vm.snapshots.push(snap.clone());
        self.store.put(tenant_id, &vm).await?;
        tracing::info!("VM snapshot taken: {} -> {}", vm.name, snap.name);
        Ok(snap)
    }

    /// Restore a VM from a snapshot (stops the VM first if running).
    pub async fn restore_snapshot(
        &mut self,
        tenant_id: &str,
        id: &str,
        snapshot_id: &str,
    ) -> anyhow::Result<()> {
        let mut vm = self.get_vm(tenant_id, id).await?;
        if !vm.snapshots.iter().any(|s| s.id == snapshot_id) {
            anyhow::bail!("snapshot {} not found on VM {}", snapshot_id, vm.name);
        }
        if vm.status == VmStatus::Running {
            self.backend.stop(&vm).await?;
        }
        self.backend.restore_snapshot(&vm, snapshot_id).await?;
        vm.status = VmStatus::Stopped;
        self.store.put(tenant_id, &vm).await?;
        tracing::info!("VM restored from snapshot: {}", vm.name);
        Ok(())
    }

    /// Clone a VM (or template) into a new VM with the given name.
    pub async fn clone_vm(
        &mut self,
        tenant_id: &str,
        id: &str,
        name: &str,
    ) -> anyhow::Result<VmConfig> {
        let source = self.get_vm(tenant_id, id).await?;
        let mut target = source.clone();
        target.id = Uuid::new_v4().to_string();
        target.name = name.to_string();
        target.status = VmStatus::Stopped;
        target.template = false;
        target.snapshots = Vec::new();
        target.disk_path = format!("/var/lib/thiscloud/vms/{}.qcow2", target.name);
        target.tenant_id = tenant_id.to_string();
        self.backend.clone(&source, &target).await?;
        self.enforce_quota(tenant_id, &target).await?;
        self.store.put(tenant_id, &target).await?;
        tracing::info!("VM cloned: {} -> {}", source.name, target.name);
        Ok(target)
    }

    /// Resize a VM; hot-paths through the backend when running.
    pub async fn resize_vm(
        &mut self,
        tenant_id: &str,
        id: &str,
        cpus: u32,
        memory_mb: u32,
    ) -> anyhow::Result<VmConfig> {
        let mut vm = self.get_vm(tenant_id, id).await?;
        let new_cpus = if cpus == 0 { vm.cpus } else { cpus };
        let new_mem = if memory_mb == 0 { vm.memory_mb } else { memory_mb };
        if vm.status == VmStatus::Running {
            self.backend.resize(&vm, new_cpus, new_mem).await?;
        }
        vm.cpus = new_cpus;
        vm.memory_mb = new_mem;
        self.store.put(tenant_id, &vm).await?;
        tracing::info!("VM resized: {} -> {} cpus / {} MB", vm.name, new_cpus, new_mem);
        Ok(vm)
    }

    /// Hot or cold attach of an extra data disk.
    pub async fn attach_disk(
        &mut self,
        tenant_id: &str,
        id: &str,
        mut disk: DiskConfig,
    ) -> anyhow::Result<DiskConfig> {
        if disk.path.is_empty() {
            anyhow::bail!("disk path is required");
        }
        if disk.id.is_empty() {
            disk.id = format!("dsk-{}", Uuid::new_v4());
        }
        let mut vm = self.get_vm(tenant_id, id).await?;
        if vm.status == VmStatus::Running {
            self.backend.attach_disk(&vm, &disk).await?;
        }
        vm.disks.push(disk.clone());
        self.store.put(tenant_id, &vm).await?;
        tracing::info!("VM disk attached: {} -> {}", vm.name, disk.path);
        Ok(disk)
    }

    /// Detach a data disk from a VM.
    pub async fn detach_disk(
        &mut self,
        tenant_id: &str,
        id: &str,
        disk_id: &str,
    ) -> anyhow::Result<()> {
        let mut vm = self.get_vm(tenant_id, id).await?;
        if vm.status == VmStatus::Running {
            self.backend.detach_disk(&vm, disk_id).await?;
        }
        vm.disks.retain(|d| d.id != disk_id);
        self.store.put(tenant_id, &vm).await?;
        tracing::info!("VM disk detached: {}", vm.name);
        Ok(())
    }

    /// Attach a NIC to a VM.
    pub async fn attach_nic(
        &mut self,
        tenant_id: &str,
        id: &str,
        tap: &str,
    ) -> anyhow::Result<()> {
        if tap.is_empty() {
            anyhow::bail!("tap is required");
        }
        let mut vm = self.get_vm(tenant_id, id).await?;
        if !vm.networks.contains(&tap.to_string()) {
            if vm.status == VmStatus::Running {
                self.backend.attach_nic(&vm, tap).await?;
            }
            vm.networks.push(tap.to_string());
            self.store.put(tenant_id, &vm).await?;
        }
        tracing::info!("VM NIC attached: {} -> {}", vm.name, tap);
        Ok(())
    }

    /// Detach a NIC from a VM.
    pub async fn detach_nic(
        &mut self,
        tenant_id: &str,
        id: &str,
        tap: &str,
    ) -> anyhow::Result<()> {
        let mut vm = self.get_vm(tenant_id, id).await?;
        if vm.status == VmStatus::Running {
            self.backend.detach_nic(&vm, tap).await?;
        }
        vm.networks.retain(|n| n != tap);
        self.store.put(tenant_id, &vm).await?;
        tracing::info!("VM NIC detached: {}", vm.name);
        Ok(())
    }

    /// Console access URL (VNC/vsock proxied by the daemon).
    pub async fn console_url(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> anyhow::Result<ConsoleInfo> {
        let vm = self.get_vm(tenant_id, id).await?;
        let url = self.backend.console_url(&vm).await?;
        Ok(ConsoleInfo { url })
    }

    // ── T1.4: HA — live migration + automatic failover ──────────────

    /// Live-migrate a VM to `target_node`. Running VMs are migrated in memory
    /// state to the target; stopped VMs only move placement (shared storage).
    /// The VM keeps its id, disks, networks and IP — only `node` and the
    /// `migrations` counter change.
    pub async fn migrate_vm(
        &mut self,
        tenant_id: &str,
        id: &str,
        target_node: &str,
    ) -> anyhow::Result<VmConfig> {
        if target_node.is_empty() {
            anyhow::bail!("target_node is required for migration");
        }
        let mut vm = self.get_vm(tenant_id, id).await?;
        // Accept a node name or id as the migration target.
        let target_node = match self.nodes.clone() {
            Some(nodes_arc) => {
                let nodes = nodes_arc.lock().await;
                nodes.resolve_id(target_node).await?
            }
            None => target_node.to_string(),
        };
        if vm.node == target_node {
            anyhow::bail!("VM {} is already placed on node {}", vm.name, target_node);
        }

        let old_node = vm.node.clone();
        match self.nodes.clone() {
            Some(nodes_arc) => {
                let mut nodes = nodes_arc.lock().await;
                // Reserve capacity on the destination first (validates online).
                nodes.reserve(&target_node, vm.cpus, vm.memory_mb).await?;
                // Live-migrate memory state when running; placement-only otherwise.
                if vm.status == VmStatus::Running {
                    if let Err(e) = self.backend.migrate(&vm, &target_node).await {
                        nodes.release(&target_node, vm.cpus, vm.memory_mb).await?;
                        return Err(e);
                    }
                }
                if !old_node.is_empty() {
                    nodes.release(&old_node, vm.cpus, vm.memory_mb).await?;
                }
            }
            None => {
                // No node registry wired (dev): just move placement.
                if vm.status == VmStatus::Running {
                    self.backend.migrate(&vm, &target_node).await?;
                }
            }
        }

        vm.node = target_node;
        vm.migrations += 1;
        self.store.put(tenant_id, &vm).await?;
        tracing::info!(
            "VM migrated: {} ({} -> {}) migrations={}",
            vm.name,
            old_node,
            vm.node,
            vm.migrations
        );
        Ok(vm)
    }

    /// Automatic HA scan: relocate every HA-enrolled running VM whose node is
    /// no longer heartbeating to a healthy node. Requires quorum (`online >=
    /// max(ha_quorum, registered/2+1)`) to avoid split-brain. Returns the ids
    /// of the VMs that were moved.
    pub async fn failover_scan(&mut self) -> anyhow::Result<Vec<String>> {
        if !self.ha_enabled || self.ha_quorum == 0 {
            return Ok(Vec::new());
        }
        let Some(nodes_arc) = self.nodes.clone() else {
            return Ok(Vec::new());
        };
        let mut nodes = nodes_arc.lock().await;
        let online = nodes.online_count().await?;
        let registered = nodes.registered_count().await?;
        let quorum = std::cmp::max(self.ha_quorum as u64, registered / 2 + 1);
        if online < quorum {
            tracing::warn!(
                "HA failover scan skipped: online={} < quorum={} (registered={})",
                online,
                quorum,
                registered
            );
            return Ok(Vec::new());
        }

        let vms = self.store.list("").await?;
        let mut moved = Vec::new();
        for mut vm in vms
            .into_iter()
            .filter(|v| v.ha && v.status == VmStatus::Running && !v.node.is_empty())
        {
            let state = nodes.node_state(&vm.node).await?;
            match state {
                // Healthy nodes keep their VMs; draining nodes are handled by
                // the operator draining the node first.
                Some(NodeState::Online) | Some(NodeState::Draining) => continue,
                // Offline (TTL expired) or gone from the registry: fail over.
                Some(NodeState::Offline) | None => {}
            }
            let target = match nodes
                .best_fit(vm.cpus, vm.memory_mb, &vm.affinity, &vm.anti_affinity)
                .await
            {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!(
                        "HA failover: no destination for VM {} ({}): {:#}",
                        vm.name,
                        vm.id,
                        e
                    );
                    continue;
                }
            };
            // Second-line guard: never relocate onto the failed node itself.
            if target == vm.node {
                continue;
            }
            if let Err(e) = nodes.reserve(&target, vm.cpus, vm.memory_mb).await {
                tracing::warn!(
                    "HA failover: could not reserve {} for VM {}: {:#}",
                    target,
                    vm.name,
                    e
                );
                continue;
            }
            let old_node = vm.node.clone();
            match self.backend.migrate(&vm, &target).await {
                Ok(()) => {
                    nodes.release(&old_node, vm.cpus, vm.memory_mb).await?;
                    vm.node = target;
                    vm.migrations += 1;
                    self.store.put(&vm.tenant_id, &vm).await?;
                    tracing::info!(
                        "HA failover: VM {} moved {} -> {} (migrations={}, ip/state preserved)",
                        vm.name,
                        old_node,
                        vm.node,
                        vm.migrations
                    );
                    moved.push(vm.id.clone());
                }
                Err(e) => {
                    nodes.release(&target, vm.cpus, vm.memory_mb).await?;
                    tracing::error!(
                        "HA failover: migrate of VM {} failed: {:#}",
                        vm.name,
                        e
                    );
                }
            }
        }
        Ok(moved)
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
