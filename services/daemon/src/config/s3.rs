use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct S3Config {
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_gateway_host")]
    pub gateway_host: String,
}

fn default_backend() -> String {
    "mock".to_string()
}

fn default_gateway_host() -> String {
    "http://127.0.0.1:7480".to_string()
}