use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ComputeConfig {
    #[serde(default = "default_http_bind")]
    pub http_bind: String,
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    #[serde(default = "default_backend")]
    pub backend: String,
}

fn default_http_bind() -> String {
    "127.0.0.1".to_string()
}

fn default_http_port() -> u16 {
    8080
}

fn default_backend() -> String {
    "mock".to_string()
}

impl Default for ComputeConfig {
    fn default() -> Self {
        Self {
            http_bind: default_http_bind(),
            http_port: default_http_port(),
            backend: default_backend(),
        }
    }
}
