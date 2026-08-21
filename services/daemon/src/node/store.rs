use crate::core::EtcdClient;
use crate::node::model::Node;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait NodeStore: Send + Sync {
    async fn put(&self, node: &Node) -> anyhow::Result<()>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<Node>>;
    async fn list(&self) -> anyhow::Result<Vec<Node>>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryNodeStore {
    nodes: Arc<Mutex<HashMap<String, Node>>>,
}

#[async_trait::async_trait]
impl NodeStore for MemoryNodeStore {
    async fn put(&self, node: &Node) -> anyhow::Result<()> {
        self.nodes.lock().unwrap().insert(node.id.clone(), node.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<Node>> {
        Ok(self.nodes.lock().unwrap().get(id).cloned())
    }

    async fn list(&self) -> anyhow::Result<Vec<Node>> {
        let mut nodes: Vec<Node> = self.nodes.lock().unwrap().values().cloned().collect();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(nodes)
    }

    async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.nodes.lock().unwrap().remove(id);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdNodeStore {
    client: EtcdClient,
}

impl EtcdNodeStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(id: &str) -> String {
        format!("/thiscloud/nodes/{}", id)
    }
}

#[async_trait::async_trait]
impl NodeStore for EtcdNodeStore {
    async fn put(&self, node: &Node) -> anyhow::Result<()> {
        let json = serde_json::to_string(node)?;
        self.client.put(&Self::key(&node.id), &json).await
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<Node>> {
        match self.client.get(&Self::key(id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<Node>> {
        let entries = self.client.list_prefix("/thiscloud/nodes/").await?;
        let mut nodes = Vec::new();
        for (_, json) in entries {
            if let Ok(node) = serde_json::from_str::<Node>(&json) {
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(id)).await
    }
}
