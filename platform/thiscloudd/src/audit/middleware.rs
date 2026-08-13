use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::audit::model::{AuditAction, AuditEntry};
use crate::audit::store::AuditStore;
use crate::auth::middleware::AuthContext;

/// Shared audit store handed to both middleware and HTTP handler.
#[derive(Clone)]
pub struct AuditState {
    pub store: Arc<Mutex<Box<dyn AuditStore>>>,
}

/// Map a request path to (resource, resource_id).
/// e.g. `/vms/123/start` → ("vms", "123"); `/networks/5` → ("networks", "5").
fn classify_path(path: &str) -> (String, String) {
    let mut parts = path.trim_start_matches('/').split('/');
    let resource = parts.next().unwrap_or("").to_string();
    let resource_id = parts.next().unwrap_or("").to_string();
    (resource, resource_id)
}

fn now_ts() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

/// Axum middleware that records Create/Update/Delete actions to the audit store.
/// Read (GET/HEAD) calls are skipped to keep the log focused on mutations.
pub async fn audit_middleware(
    State(state): State<AuditState>,
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let auth = req.extensions().get::<AuthContext>().cloned();

    let resp = next.run(req).await;

    let action = match method.as_str() {
        "POST" => Some(AuditAction::Create),
        "PUT" | "PATCH" => Some(AuditAction::Update),
        "DELETE" => Some(AuditAction::Delete),
        _ => None,
    };

    if let Some(action) = action {
        // Only log 2xx outcomes.
        if resp.status().is_success() {
            let username = auth.as_ref().map(|a| a.sub().to_string()).unwrap_or_default();
            let role = auth
                .as_ref()
                .map(|a| format!("{:?}", a.claims.role).to_lowercase())
                .unwrap_or_default();
            let tenant_id = auth
                .as_ref()
                .map(|a| a.tenant_id().to_string())
                .unwrap_or_default();

            let (resource, resource_id) = classify_path(&path);
            let entry = AuditEntry {
                id: Uuid::new_v4().to_string(),
                timestamp: now_ts(),
                user: username,
                role,
                tenant_id,
                action,
                resource,
                resource_id,
                detail: method.as_str().to_string(),
            };
            state.store.lock().await.log(entry).await;
        }
    }

    resp
}