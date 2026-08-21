use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MarketplaceConfig {
    #[serde(default = "default_backend")]
    pub backend: String,
}

fn default_backend() -> String {
    "mock".to_string()
}
