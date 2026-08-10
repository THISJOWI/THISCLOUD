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
}
