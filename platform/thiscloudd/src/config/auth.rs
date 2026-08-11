use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Enable JWT authentication on all endpoints
    #[serde(default)]
    pub enabled: bool,
    /// HMAC-SHA256 secret for JWT signing
    pub jwt_secret: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: None,
        }
    }
}
