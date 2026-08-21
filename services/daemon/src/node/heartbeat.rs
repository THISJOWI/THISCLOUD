use super::model::{Node, NodeHeartbeat};
use super::NodeModule;

/// Periodic agent loop for the local daemon.
///
/// When `masters` is non-empty the daemon acts as a registered cluster agent:
/// it POSTs heartbeats to the first reachable master (rotating on failure) and
/// re-registers if a master no longer knows the node. Otherwise the local node
/// store is refreshed in place (master / single node).
pub struct SelfHeartbeat {
    node: Node,
    masters: Vec<String>,
    client: reqwest::Client,
    master_idx: usize,
    fail_streak: u32,
}

impl SelfHeartbeat {
    pub fn new(node: Node, masters: Vec<String>) -> Self {
        Self {
            node,
            masters,
            client: reqwest::Client::new(),
            master_idx: 0,
            fail_streak: 0,
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node.id
    }

    /// Number of consecutive failed beat attempts (drives backoff).
    pub fn fail_streak(&self) -> u32 {
        self.fail_streak
    }

    async fn register_with(&self, base: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/v1/nodes", base.trim_end_matches('/'));
        let resp = self.client.post(&url).json(&self.node).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("master rejected registration: {}", resp.status());
        }
        Ok(())
    }

    /// Upsert registration, then retry the heartbeat once (used when a master
    /// replies 404 — e.g. a freshly started master that lost its store).
    async fn re_register_and_beat(
        &self,
        base: &str,
        url: &str,
        usage: &NodeHeartbeat,
    ) -> anyhow::Result<()> {
        self.register_with(base).await?;
        let retry = self.client.post(url).json(usage).send().await?;
        if !retry.status().is_success() {
            anyhow::bail!("heartbeat rejected after re-register: {}", retry.status());
        }
        Ok(())
    }

    /// Refresh liveness for this daemon's node entry.
    ///
    /// `module` is only used for the local path (empty `masters`); the remote
    /// path never touches it so callers can pass `None` and avoid holding the
    /// node store lock across the HTTP call.
    pub async fn beat(
        &mut self,
        module: Option<&mut NodeModule>,
        usage: NodeHeartbeat,
    ) -> anyhow::Result<()> {
        if self.masters.is_empty() {
            let module = module
                .ok_or_else(|| anyhow::anyhow!("local heartbeat needs the node store"))?;
            module
                .heartbeat(&self.node.id, usage)
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!("local node {} is not registered", self.node.id)
                })?;
            self.fail_streak = 0;
            return Ok(());
        }

        let n = self.masters.len();
        for _ in 0..n {
            let base = &self.masters[self.master_idx % n];
            let url = format!(
                "{}/api/v1/nodes/{}/heartbeat",
                base.trim_end_matches('/'),
                self.node.id
            );
            match self.client.post(&url).json(&usage).send().await {
                Ok(resp) if resp.status().is_success() => {
                    self.fail_streak = 0;
                    return Ok(());
                }
                Ok(resp) if resp.status() == reqwest::StatusCode::NOT_FOUND => {
                    // Master doesn't know this node yet (fresh master) — upsert
                    // the registration then retry the heartbeat once.
                    if let Ok(()) = self.re_register_and_beat(base, &url, &usage).await {
                        self.fail_streak = 0;
                        return Ok(());
                    }
                }
                _ => {}
            }
            self.master_idx += 1;
            self.fail_streak = self.fail_streak.saturating_add(1);
        }
        anyhow::bail!("no master reachable (tried {} masters)", n)
    }
}