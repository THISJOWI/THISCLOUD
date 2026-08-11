use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Enable JWT authentication on all endpoints
    #[serde(default)]
    pub enabled: bool,
    /// HMAC-SHA256 secret for JWT signing
    pub jwt_secret: Option<String>,
    /// JWT token time-to-live in seconds (default: 86400 = 24h)
    #[serde(default = "default_jwt_ttl")]
    pub jwt_ttl_secs: u64,
    /// TLS configuration for HTTPS
    #[serde(default)]
    pub tls: TlsConfig,
}

fn default_jwt_ttl() -> u64 {
    86400
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: None,
            jwt_ttl_secs: default_jwt_ttl(),
            tls: TlsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS on the HTTP listener
    #[serde(default)]
    pub enabled: bool,
    /// Path to the PEM-encoded certificate chain
    pub cert_path: Option<String>,
    /// Path to the PEM-encoded private key
    pub key_path: Option<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_path: None,
            key_path: None,
        }
    }
}
