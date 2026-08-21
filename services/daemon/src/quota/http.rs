use crate::quota::model::TenantQuota;
use crate::quota::module::QuotaModule;
use crate::core::AppError;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct QuotaApiState {
    pub module: Arc<Mutex<QuotaModule>>,
}

impl QuotaApiState {
    pub fn new(module: Arc<Mutex<QuotaModule>>) -> Self {
        Self { module }
    }
}

pub fn app(state: QuotaApiState) -> Router {
    Router::new()
        .route("/quotas", get(list_quotas))
        .route(
            "/quotas/:tenant_id",
            get(get_quota).put(set_quota).delete(delete_quota),
        )
        .with_state(state)
}

async fn list_quotas(
    State(state): State<QuotaApiState>,
) -> Result<Json<Vec<TenantQuota>>, AppError> {
    let module = state.module.lock().await;
    Ok(Json(module.list().await?))
}

async fn get_quota(
    State(state): State<QuotaApiState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<TenantQuota>, AppError> {
    let module = state.module.lock().await;
    Ok(Json(module.get(&tenant_id).await?))
}

async fn set_quota(
    State(state): State<QuotaApiState>,
    Path(tenant_id): Path<String>,
    Json(mut quota): Json<TenantQuota>,
) -> Result<impl IntoResponse, AppError> {
    quota.tenant_id = tenant_id;
    let module = state.module.lock().await;
    module.set(quota.clone()).await?;
    Ok((StatusCode::OK, Json(quota)))
}

async fn delete_quota(
    State(state): State<QuotaApiState>,
    Path(tenant_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let module = state.module.lock().await;
    module.delete(&tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn router() -> Router {
        let module = QuotaModule::with_memory_store();
        app(QuotaApiState::new(Arc::new(Mutex::new(module))))
    }

    #[tokio::test]
    async fn set_and_get_quota() {
        let r = router();
        let body = r#"{"tenant_id":"t1","max_cpus":4,"max_vms":2,"max_memory_mb":8192,"max_storage_gb":100,"max_networks":3}"#;
        let resp = r
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/quotas/t1")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let resp = r
            .oneshot(
                Request::builder()
                    .uri("/quotas/t1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let q: TenantQuota = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(q.max_cpus, 4);
        assert_eq!(q.max_vms, 2);
    }

    #[tokio::test]
    async fn list_quotas_empty() {
        let r = router();
        let resp = r
            .oneshot(
                Request::builder()
                    .uri("/quotas")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let list: Vec<TenantQuota> = serde_json::from_slice(&bytes).unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn delete_quota_returns_204() {
        let r = router();
        let body = r#"{"tenant_id":"t2","max_cpus":1,"max_memory_mb":1,"max_vms":1,"max_storage_gb":1,"max_networks":1}"#;
        r.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/quotas/t2")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let resp = r
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/quotas/t2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}