use etcd_client::{Client as EtcdRawClient, DeleteOptions, GetOptions, PutOptions};

#[derive(Clone)]
pub struct EtcdClient {
    client: EtcdRawClient,
    endpoints: Vec<String>,
}

impl EtcdClient {
    pub async fn connect(endpoints: &str) -> anyhow::Result<Self> {
        let client = EtcdRawClient::connect([endpoints], None).await?;
        Ok(Self {
            client,
            endpoints: vec![endpoints.to_string()],
        })
    }

    pub fn endpoints(&self) -> &[String] {
        &self.endpoints
    }

    /// Probe the cluster via the maintenance status RPC. A successful status
    /// round-trip requires a live cluster (leader elected); etcd only answers
    /// status from a healthy member, so this is the connectivity signal.
    pub async fn healthy(&self) -> bool {
        let client = self.client.clone();
        client
            .maintenance_client()
            .status()
            .await
            .map(|s| s.header().is_some())
            .unwrap_or(false)
    }

    pub async fn put(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.client.clone().put(key, value, Some(PutOptions::new())).await?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        let resp = self.client.clone().get(key, Some(GetOptions::new())).await?;
        Ok(resp
            .kvs()
            .first()
            .map(|kv| String::from_utf8_lossy(kv.value()).to_string()))
    }

    pub async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.client.clone().delete(key, Some(DeleteOptions::new())).await?;
        Ok(())
    }

    /// List all key-value pairs under a prefix.
    pub async fn list_prefix(&self, prefix: &str) -> anyhow::Result<Vec<(String, String)>> {
        let resp = self
            .client
            .clone()
            .get(prefix, Some(GetOptions::new().with_all_keys().with_prefix()))
            .await?;
        let mut out = Vec::new();
        for kv in resp.kvs() {
            let k = String::from_utf8_lossy(kv.key()).to_string();
            let v = String::from_utf8_lossy(kv.value()).to_string();
            out.push((k, v));
        }
        Ok(out)
    }

    /// Dump every key/value pair in the cluster. Backups capture the full
    /// etcd state so a restore reconstructs the exact cluster configuration.
    pub async fn dump(&self) -> anyhow::Result<Vec<(String, String)>> {
        self.list_prefix("").await
    }

    /// Delete every key/value pair in the cluster. Used by restore to return
    /// to a clean slate before replaying a snapshot.
    pub async fn wipe(&self) -> anyhow::Result<()> {
        self.client
            .clone()
            .delete(
                "",
                Some(DeleteOptions::new().with_all_keys().with_prefix()),
            )
            .await?;
        Ok(())
    }

    /// Write every key/value pair. Used by restore to replay a snapshot.
    pub async fn write_all(&self, entries: &[(String, String)]) -> anyhow::Result<()> {
        for (k, v) in entries {
            self.put(k, v).await?;
        }
        Ok(())
    }
}
