use crate::auth::middleware::init_secret;
use crate::auth::model::{Claims, Role};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Shared login state holding the JWT secret and TTL.
#[derive(Clone)]
pub struct LoginState {
    secret: Arc<String>,
    ttl_secs: u64,
}

impl LoginState {
    pub fn new(secret: String, ttl_secs: u64) -> Self {
        Self {
            secret: Arc::new(secret),
            ttl_secs,
        }
    }
}

pub fn ensure_secret(secret: &str) {
    let _ = init_secret(secret.to_string());
}

pub fn router(state: LoginState) -> Router {
    Router::new()
        .route("/auth/login", post(login))
        .with_state(state)
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub role: String,
    pub tenant_id: String,
    pub expires_at: u64,
}

#[derive(Debug)]
pub enum LoginError {
    MissingFields,
    InvalidCredentials,
    AuthCommandFailed(String),
    Internal(String),
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        match self {
            LoginError::MissingFields => {
                (StatusCode::BAD_REQUEST, "username and password required").into_response()
            }
            LoginError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "invalid credentials").into_response()
            }
            LoginError::AuthCommandFailed(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("auth command failed: {msg}"))
                    .into_response()
            }
            LoginError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
            }
        }
    }
}

async fn login(
    State(state): State<LoginState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, LoginError> {
    if req.username.is_empty() || req.password.is_empty() {
        return Err(LoginError::MissingFields);
    }

    // Validate username: only safe chars, no shell injection.
    if !req
        .username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        || req.username.len() > 32
    {
        return Err(LoginError::InvalidCredentials);
    }

    let ok = pam_authenticate(&req.username, &req.password)
        .await
        .map_err(LoginError::AuthCommandFailed)?;
    if !ok {
        return Err(LoginError::InvalidCredentials);
    }

    let role = check_wheel(&req.username).await.unwrap_or(false);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let exp = now + state.ttl_secs as usize;

    let claims = Claims {
        sub: req.username,
        tenant_id: String::new(),
        role: if role { Role::Admin } else { Role::TenantUser },
        exp,
        iat: now,
    };

    let token =
        crate::auth::middleware::encode_jwt(&claims, &state.secret)
            .map_err(|e| LoginError::Internal(format!("jwt encode: {e}")))?;

    let role_str = match claims.role {
        Role::Admin => "admin".to_string(),
        _ => "tenant_user".to_string(),
    };

    Ok(Json(LoginResponse {
        token,
        role: role_str,
        tenant_id: claims.tenant_id,
        expires_at: exp as u64,
    }))
}

/// Verify password via `su`. Returns true if credentials are valid.
async fn pam_authenticate(username: &str, password: &str) -> Result<bool, String> {
    let mut child = tokio::process::Command::new("su")
        .args(["-c", "echo ok", username])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn su: {e}"))?;

    use tokio::io::AsyncWriteExt;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(password.as_bytes()).await;
        let _ = stdin.write_all(b"\n").await;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("su wait: {e}"))?;

    Ok(output.status.success())
}

/// Check if user is in the wheel or admin group.
async fn check_wheel(username: &str) -> Result<bool, String> {
    let output = tokio::process::Command::new("id")
        .args(["-nG", username])
        .output()
        .await
        .map_err(|e| format!("id failed: {e}"))?;

    if !output.status.success() {
        return Ok(false);
    }

    let groups = String::from_utf8_lossy(&output.stdout);
    Ok(groups
        .split_whitespace()
        .any(|g| g == "wheel" || g == "admin"))
}
