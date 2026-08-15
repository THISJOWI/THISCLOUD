use super::model::NodeHeartbeat;
use super::NodeModule;

/// Periodic self-heartbeat for the local daemon.
///
/// When `master` is set, heartbeats are POSTed to the master daemon's
/// `/api/v1/nodes/{id}/heartbeat` endpoint (worker in a split-store cluster);
/// otherwise the local node store is updated in place (master / single node).
pub struct SelfHeartbeat {
    pub node_id: String,
    pub master: Option<String>,
    client: reqwest::Client,
}

impl SelfHeartbeat {
    pub fn new(node_id: String, master: Option<String>) -> Self {
        Self {
            node_id,
            master,
            client: reqwest::Client::new(),
        }
    }

    /// Refresh liveness for this daemon's node entry. Returns an error when
    /// the node cannot be heartbeated at all (not registered / master down).
    ///
    /// `module` is only used for the local path (no `master`); the remote path
    /// never touches it so callers can pass `None` and avoid holding the node
    /// store lock across the HTTP call.
    pub async fn beat(&self, module: Option<&mut NodeModule>, usage: NodeHeartbeat) -> anyhow::Result<()> {
        match &self.master {
            None => {
                let module = module
                    .ok_or_else(|| anyhow::anyhow!("local heartbeat needs the node store"))?;
                module
                    .heartbeat(&self.node_id, usage)
                    .await?
                    .ok_or_else(|| {
                        anyhow::anyhow!("local node {} is not registered", self.node_id)
                    })?;
                Ok(())
            }
            Some(base) => {
                let url = format!(
                    "{}/api/v1/nodes/{}/heartbeat",
                    base.trim_end_matches('/'),
                    self.node_id
                );
                let resp = self.client.post(&url).json(&usage).send().await?;
                if !resp.status().is_success() {
                    anyhow::bail!("master rejected heartbeat: {}", resp.status());
                }
                Ok(())
            }
        }
    }
}