use serde::{Deserialize, Serialize};

/// Disk/artifact formats the image registry understands.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ImageFormat {
    #[serde(rename = "qcow2")]
    #[default]
    Qcow2,
    #[serde(rename = "iso")]
    Iso,
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "cloud-init")]
    CloudInit,
}

impl ImageFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageFormat::Qcow2 => "qcow2",
            ImageFormat::Iso => "iso",
            ImageFormat::Raw => "raw",
            ImageFormat::CloudInit => "cloud-init",
        }
    }
}

/// OS family, used to pick kernel/initrd and cloud-init behaviour.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum OsFamily {
    #[serde(rename = "generic")]
    #[default]
    Generic,
    #[serde(rename = "alma")]
    Alma,
    #[serde(rename = "ubuntu")]
    Ubuntu,
    #[serde(rename = "debian")]
    Debian,
    #[serde(rename = "fedora")]
    Fedora,
    #[serde(rename = "rocky")]
    Rocky,
}

impl OsFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            OsFamily::Generic => "generic",
            OsFamily::Alma => "alma",
            OsFamily::Ubuntu => "ubuntu",
            OsFamily::Debian => "debian",
            OsFamily::Fedora => "fedora",
            OsFamily::Rocky => "rocky",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageStatus {
    #[default]
    Available,
    Importing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Image {
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// Where the artifact came from: an HTTP(S) URL or a local pool path.
    pub source: String,
    /// Digest (sha256) computed at import time; empty for CloudInit profiles.
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub format: ImageFormat,
    #[serde(default)]
    pub os_family: OsFamily,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub template: bool,
    #[serde(default)]
    pub status: ImageStatus,
    #[serde(default)]
    pub tenant_id: String,
}

impl Image {
    pub fn new(name: String, source: String, format: ImageFormat) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            source,
            sha256: String::new(),
            size_bytes: 0,
            format,
            os_family: OsFamily::Generic,
            version: String::new(),
            template: false,
            status: ImageStatus::Available,
            tenant_id: String::new(),
        }
    }
}