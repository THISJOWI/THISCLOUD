use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::http::{HeaderMap, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::model::Claims;

/// Auth context injected into every request by the middleware.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub claims: Claims,
}

impl AuthContext {
    pub fn tenant_id(&self) -> &str {
        &self.claims.tenant_id
    }
    pub fn sub(&self) -> &str {
        &self.claims.sub
    }
}

/// Shared JWT secret. Loaded once from config/env at daemon start.
use std::sync::OnceLock;
static JWT_SECRET: OnceLock<String> = OnceLock::new();

/// Initialize the JWT secret (call once at startup).
pub fn init_secret(secret: String) {
    let _ = JWT_SECRET.set(secret);
}

fn get_secret() -> Result<&'static str, AuthError> {
    JWT_SECRET
        .get()
        .map(|s| s.as_str())
        .ok_or(AuthError::SecretNotConfigured)
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    SecretNotConfigured,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::MissingToken => {
                (StatusCode::UNAUTHORIZED, "missing authorization header").into_response()
            }
            AuthError::InvalidToken(msg) => {
                let body = format!("{{\"error\":\"{msg}\"}}");
                (StatusCode::UNAUTHORIZED, body).into_response()
            }
            AuthError::SecretNotConfigured => {
                (StatusCode::INTERNAL_SERVER_ERROR, "jwt secret not configured").into_response()
            }
        }
    }
}

/// Axum middleware function — enforces JWT Bearer auth.
/// Use with: `axum::middleware::from_fn(jwt_auth)`
pub async fn jwt_auth(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let token = extract_bearer(req.headers())?;
    let secret = get_secret()?;
    let claims = decode_jwt(token, secret).map_err(AuthError::InvalidToken)?;

    req.extensions_mut().insert(AuthContext { claims });

    Ok(next.run(req).await)
}

/// Axum extractor that pulls the tenant_id from the auth context.
/// Falls back to empty string (global scope) when auth middleware is disabled.
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: String,
}

#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for TenantContext {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let tenant_id = parts
            .extensions
            .get::<AuthContext>()
            .map(|ctx| ctx.tenant_id().to_string())
            .unwrap_or_default();
        Ok(TenantContext { tenant_id })
    }
}

/// Extract Bearer token from Authorization header.
fn extract_bearer(headers: &HeaderMap) -> Result<&str, AuthError> {
    let header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::MissingToken)
}

/// Decode and verify a JWT (HMAC-SHA256).
fn decode_jwt(token: &str, secret: &str) -> Result<Claims, String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("invalid jwt format".into());
    }

    let (header_b64, payload_b64, sig_b64) = (parts[0], parts[1], parts[2]);

    // Verify signature
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|e| e.to_string())?;
    mac.update(header_b64.as_bytes());
    mac.update(b".");
    mac.update(payload_b64.as_bytes());
    let sig = base64_url_decode(sig_b64).map_err(|e| e.to_string())?;
    mac.verify_slice(&sig)
        .map_err(|_| "invalid signature".to_string())?;

    // Decode payload
    let payload_json =
        base64_url_decode(payload_b64).map_err(|e| format!("base64 decode: {e}"))?;
    let claims: Claims =
        serde_json::from_slice(&payload_json).map_err(|e| format!("json parse: {e}"))?;

    // Check expiration
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    if claims.exp < now {
        return Err("token expired".into());
    }

    Ok(claims)
}

/// Encode a JWT token with HMAC-SHA256.
pub fn encode_jwt(claims: &Claims, secret: &str) -> Result<String, String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let header_b64 = base64_url_encode(header.as_bytes());
    let payload_json =
        serde_json::to_string(claims).map_err(|e| format!("json serialize: {e}"))?;
    let payload_b64 = base64_url_encode(payload_json.as_bytes());

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|e| e.to_string())?;
    mac.update(header_b64.as_bytes());
    mac.update(b".");
    mac.update(payload_b64.as_bytes());
    let sig = base64_url_encode(&mac.finalize().into_bytes());

    Ok(format!("{header_b64}.{payload_b64}.{sig}"))
}

/// Base64url encode (no padding).
fn base64_url_encode(input: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(input)
}

/// Base64url decode (no padding).
fn base64_url_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::model::Role;

    #[test]
    fn test_jwt_roundtrip() {
        init_secret("test-secret-key".into());
        let claims = Claims {
            sub: "user-1".into(),
            tenant_id: "tenant-a".into(),
            role: Role::Admin,
            exp: 9999999999,
            iat: 1000000000,
        };
        let token = encode_jwt(&claims, "test-secret-key").unwrap();
        let decoded = decode_jwt(&token, "test-secret-key").unwrap();
        assert_eq!(decoded.sub, "user-1");
        assert_eq!(decoded.tenant_id, "tenant-a");
        assert_eq!(decoded.role, Role::Admin);
    }

    #[test]
    fn test_jwt_rejects_bad_signature() {
        init_secret("test-secret-key".into());
        let claims = Claims {
            sub: "user-1".into(),
            tenant_id: "tenant-a".into(),
            role: Role::Admin,
            exp: 9999999999,
            iat: 1000000000,
        };
        let token = encode_jwt(&claims, "test-secret-key").unwrap();
        let result = decode_jwt(&token, "wrong-secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_jwt_rejects_expired() {
        init_secret("test-secret-key".into());
        let claims = Claims {
            sub: "user-1".into(),
            tenant_id: "tenant-a".into(),
            role: Role::Admin,
            exp: 1000000000, // 2001-09-09
            iat: 999999999,
        };
        let token = encode_jwt(&claims, "test-secret-key").unwrap();
        let result = decode_jwt(&token, "test-secret-key");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expired"));
    }
}
