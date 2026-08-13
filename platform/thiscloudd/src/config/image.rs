use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ImageConfig {
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_images_dir")]
    pub images_dir: String,
}

fn default_backend() -> String {
    "mock".to_string()
}

fn default_images_dir() -> String {
    "/var/lib/thiscloud/images".to_string()
}