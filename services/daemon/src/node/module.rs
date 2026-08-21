use crate::node::model::{Node, NodeHeartbeat, NodeRole, NodeState};
use crate::node::store::{MemoryNodeStore, NodeStore};
use uuid::Uuid;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub struct NodeModule {
    store: Box<dyn NodeStore>,
}

impl NodeModule {
    pub fn new(store: Box<dyn NodeStore>) -> Self {
        Self { store }
    }

    pub fn with_memory_store() -> Self {
        Self::new(Box::new(MemoryNodeStore::default()))
    }

    /// Effective state: a node that hasn't heartbeated within its TTL is offline.
    pub fn effective_state(node: &Node) -> NodeState {
        if node.state == NodeState::Draining {
            return NodeState::Draining;
        }
        let elapsed = now_secs().saturating_sub(node.last_seen_secs);
        if node.last_seen_secs == 0 || elapsed > node.ttl_secs {
            NodeState::Offline
        } else {
            NodeState::Online
        }
    }

    pub async fn register(&mut self, mut node: Node) -> anyhow::Result<Node> {
        if node.id.is_empty() {
            node.id = format!("node-{}", Uuid::new_v4());
        }
        if node.hostname.is_empty() {
            node.hostname = node.name.clone();
        }
        node.last_seen_secs = now_secs();
        if node.ttl_secs == 0 {
            node.ttl_secs = 30;
        }
        self.store.put(&node).await?;
        tracing::info!("Node registered: {} ({}) role={:?}", node.name, node.id, node.role);
        Ok(node)
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Node>> {
        self.store.get(id).await
    }

    pub async fn list(&self) -> anyhow::Result<Vec<Node>> {
        let mut nodes = self.store.list().await?;
        for n in nodes.iter_mut() {
            n.state = Self::effective_state(n);
        }
        nodes.sort_by_key(|n| n.name.clone());
        Ok(nodes)
    }

    pub async fn is_empty(&self) -> anyhow::Result<bool> {
        Ok(self.store.list().await?.is_empty())
    }

    /// Number of nodes currently heartbeating within their TTL (HA quorum input).
    pub async fn online_count(&self) -> anyhow::Result<u64> {
        Ok(self
            .store
            .list()
            .await?
            .iter()
            .filter(|n| Self::effective_state(n) == NodeState::Online)
            .count() as u64)
    }

    /// Total number of registered nodes (HA quorum denominator).
    pub async fn registered_count(&self) -> anyhow::Result<u64> {
        Ok(self.store.list().await?.len() as u64)
    }

    /// Effective (TTL-based) state of a single node.
    pub async fn node_state(&self, id: &str) -> anyhow::Result<Option<NodeState>> {
        Ok(self.store.get(id).await?.map(|n| Self::effective_state(&n)))
    }

    pub async fn delete(&mut self, id: &str) -> anyhow::Result<()> {
        self.store.delete(id).await
    }

    /// Agent heartbeat: refresh liveness and reported usage.
    pub async fn heartbeat(&mut self, id: &str, hb: NodeHeartbeat) -> anyhow::Result<Option<Node>> {
        let mut node = match self.store.get(id).await? {
            Some(n) => n,
            None => return Ok(None),
        };
        node.cpus_used = hb.cpus_used;
        node.memory_used_mb = hb.memory_used_mb;
        node.vms = hb.vms;
        node.last_seen_secs = now_secs();
        if node.state != NodeState::Draining {
            node.state = NodeState::Online;
        }
        self.store.put(&node).await?;
        Ok(Some(node))
    }

    /// Start or stop draining a node. Draining nodes are excluded from scheduling.
    pub async fn drain(&mut self, id: &str, drain: bool) -> anyhow::Result<Option<Node>> {
        let mut node = match self.store.get(id).await? {
            Some(n) => n,
            None => return Ok(None),
        };
        node.state = if drain {
            NodeState::Draining
        } else {
            Self::effective_state(&node)
        };
        self.store.put(&node).await?;
        Ok(Some(node))
    }

    /// Best-fit scheduling: online, non-draining, capacity- and affinity-compatible
    /// nodes sorted by lowest load. Returns the node id.
    pub async fn best_fit(
        &self,
        cpus: u32,
        memory_mb: u32,
        affinity: &[String],
        anti_affinity: &[String],
    ) -> anyhow::Result<String> {
        let mut candidates: Vec<Node> = self
            .store
            .list()
            .await?
            .into_iter()
            .filter(|n| {
                let state = Self::effective_state(n);
                state != NodeState::Draining && state != NodeState::Offline
            })
            .filter(|n| n.is_capable(cpus, memory_mb))
            .filter(|n| {
                if affinity.is_empty() {
                    return true;
                }
                n.labels.iter().any(|l| affinity.contains(l))
            })
            .filter(|n| {
                if anti_affinity.is_empty() {
                    return true;
                }
                !n.labels.iter().any(|l| anti_affinity.contains(l))
            })
            .collect();

        candidates.sort_by(|a, b| {
            a.load_ratio()
                .partial_cmp(&b.load_ratio())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cpus_used.cmp(&b.cpus_used))
        });

        candidates
            .first()
            .map(|n| n.id.clone())
            .ok_or_else(|| anyhow::anyhow!("no suitable node for scheduling (resources or affinity)"))
    }

    /// Resolve a node reference (id or name) to its canonical node id.
    /// Callers may specify a node by either its `id` or its `name`.
    pub async fn resolve_id(&self, id_or_name: &str) -> anyhow::Result<String> {
        if let Some(n) = self.store.get(id_or_name).await? {
            return Ok(n.id);
        }
        let nodes = self.store.list().await?;
        nodes
            .into_iter()
            .find(|n| n.name == id_or_name)
            .map(|n| n.id)
            .ok_or_else(|| anyhow::anyhow!("node '{}' not found (by id or name)", id_or_name))
    }

    /// Reserve capacity on a node when a VM is placed on it. The node may be
    /// referenced by its id or its name.
    pub async fn reserve(&mut self, node_id: &str, cpus: u32, memory_mb: u32) -> anyhow::Result<()> {
        let node_id = self.resolve_id(node_id).await?;
        let mut node = self
            .store
            .get(&node_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("node {} not found", node_id))?;
        let state = Self::effective_state(&node);
        if state != NodeState::Online {
            anyhow::bail!("node {} is not online (state={:?})", node_id, state);
        }
        if !node.is_capable(cpus, memory_mb) {
            anyhow::bail!(
                "node {} lacks capacity for {} cpus / {} MB",
                node_id,
                cpus,
                memory_mb
            );
        }
        node.cpus_used += cpus;
        node.memory_used_mb += memory_mb;
        node.vms += 1;
        self.store.put(&node).await?;
        Ok(())
    }

    /// Free capacity on a node when a VM placed on it is deleted. The node may be
    /// referenced by its id or its name.
    pub async fn release(&mut self, node_id: &str, cpus: u32, memory_mb: u32) -> anyhow::Result<()> {
        let node_id = match self.resolve_id(node_id).await {
            Ok(id) => id,
            Err(_) => return Ok(()), // unknown node: nothing to release
        };
        let mut node = match self.store.get(&node_id).await? {
            Some(n) => n,
            None => return Ok(()),
        };
        node.cpus_used = node.cpus_used.saturating_sub(cpus);
        node.memory_used_mb = node.memory_used_mb.saturating_sub(memory_mb);
        node.vms = node.vms.saturating_sub(1);
        self.store.put(&node).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::core::Module for NodeModule {
    fn name(&self) -> &str {
        "node"
    }

    async fn start(&mut self, _event_bus: &crate::core::EventBus) -> anyhow::Result<()> {
        tracing::info!("Node module started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Node module stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}

impl NodeModule {
    pub async fn seed_local_master(&mut self) -> anyhow::Result<Node> {
        let cpus = std::thread::available_parallelism()
            .map(|p| p.get() as u32)
            .unwrap_or(1);
        let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string());
        self.register(Node {
            id: "master-1".to_string(),
            name: "master".to_string(),
            role: NodeRole::Master,
            address: "127.0.0.1:8080".to_string(),
            hostname,
            cpus_total: cpus,
            cpus_used: 0,
            memory_total_mb: 0,
            memory_used_mb: 0,
            vms: 0,
            state: NodeState::Online,
            last_seen_secs: now_secs(),
            ttl_secs: 0,
            labels: Vec::new(),
        })
        .await
    }
}
