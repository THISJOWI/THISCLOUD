use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppType {
    #[serde(rename = "iso")]
    Iso,
    #[serde(rename = "docker")]
    DockerImage,
    #[serde(rename = "cloud-init")]
    CloudInit,
    #[serde(rename = "turbokit")]
    TurboKit,
}

impl AppType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppType::Iso => "iso",
            AppType::DockerImage => "docker",
            AppType::CloudInit => "cloud-init",
            AppType::TurboKit => "turbokit",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum MarketplaceStatus {
    #[default]
    Installed,
    NotInstalled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketplaceApp {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub app_type: AppType,
    pub source: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub status: MarketplaceStatus,
}

impl MarketplaceApp {
    pub fn new(
        name: String,
        app_type: AppType,
        source: String,
        version: String,
        description: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            app_type,
            source,
            version,
            description,
            status: MarketplaceStatus::NotInstalled,
        }
    }
}
