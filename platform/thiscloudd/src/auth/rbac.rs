//! T0.3 RBAC: role-based access control on top of JWT auth.
//!
//! `RequireRole<T>` is an extractor (usable directly or as a route layer via
//! `axum::middleware::from_extractor`) that rejects with 403 when the
//! authenticated role is not in the allowed set for that route. When no
//! `AuthContext` is present (auth disabled / dev mode) access is allowed,
//! matching the `TenantContext` fallback.

use std::marker::PhantomData;

use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use super::middleware::AuthContext;
use super::model::Role;

// ---------------------------------------------------------------------------
// Role sets
// ---------------------------------------------------------------------------

/// Roles allowed on any authenticated route (read-only access).
pub const ALL_ROLES: &[Role] = &[
    Role::Admin,
    Role::Operator,
    Role::TenantAdmin,
    Role::TenantUser,
    Role::Auditor,
];

/// Roles allowed to mutate resources (delete, start/stop).
pub const WRITE_ROLES: &[Role] = &[Role::Admin, Role::Operator, Role::TenantAdmin];

/// Roles allowed to create tenant resources (TenantUser may create own).
pub const CREATE_ROLES: &[Role] = &[
    Role::Admin,
    Role::Operator,
    Role::TenantAdmin,
    Role::TenantUser,
];

/// Roles allowed to modify auth settings.
pub const AUTH_MANAGE_ROLES: &[Role] = &[Role::Admin];

/// Static role sets carried by `RequireRole<T>` as type-level markers.
pub trait RoleSet {
    const ALLOWED: &'static [Role];
}

/// Marker: all authenticated roles (read routes).
pub struct ReadSet;
/// Marker: Admin, Operator, TenantAdmin (mutate/delete/start/stop).
pub struct WriteSet;
/// Marker: Admin, Operator, TenantAdmin, TenantUser (create).
pub struct CreateSet;
/// Marker: Admin only (auth settings).
pub struct AuthManageSet;

impl RoleSet for ReadSet {
    const ALLOWED: &'static [Role] = ALL_ROLES;
}
impl RoleSet for WriteSet {
    const ALLOWED: &'static [Role] = WRITE_ROLES;
}
impl RoleSet for CreateSet {
    const ALLOWED: &'static [Role] = CREATE_ROLES;
}
impl RoleSet for AuthManageSet {
    const ALLOWED: &'static [Role] = AUTH_MANAGE_ROLES;
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Rejection returned by `RequireRole` when the role is not permitted.
#[derive(Debug)]
pub enum RoleError {
    /// The authenticated role is not in the allowed set for this route.
    Forbidden,
}

impl IntoResponse for RoleError {
    fn into_response(self) -> Response {
        (StatusCode::FORBIDDEN, "forbidden: insufficient role").into_response()
    }
}

// ---------------------------------------------------------------------------
// Typed extractor  (used in route_layer via from_extractor)
// ---------------------------------------------------------------------------

/// Axum extractor enforcing that the authenticated role is in `T::ALLOWED`.
///
/// Bypassed (allowed) when no `AuthContext` is present — i.e. auth is disabled.
///
/// ```ignore
/// use axum::middleware;
/// use crate::auth::rbac::{RequireRole, WriteSet};
///
/// // reject with 403 when the role is not in WriteSet::ALLOWED
/// .route_layer(middleware::from_extractor::<RequireRole<WriteSet>>())
/// ```
pub struct RequireRole<T> {
    _marker: PhantomData<T>,
}

#[axum::async_trait]
impl<S, T> FromRequestParts<S> for RequireRole<T>
where
    S: Send + Sync,
    T: RoleSet,
{
    type Rejection = RoleError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match parts.extensions.get::<AuthContext>() {
            // Auth disabled → allow (dev mode), same as TenantContext fallback.
            None => Ok(Self {
                _marker: PhantomData,
            }),
            Some(ctx) if T::ALLOWED.contains(&ctx.claims.role) => Ok(Self {
                _marker: PhantomData,
            }),
            Some(_) => Err(RoleError::Forbidden),
        }
    }
}

// ---------------------------------------------------------------------------
// Middleware function (closure-based, accepts runtime role list)
// ---------------------------------------------------------------------------

/// Middleware function requiring the role to be in a set.
///
/// Use via `axum::middleware::from_fn(require_role_mw).with_state(Arc::new(vec![...]))` or
/// the typed `RequireRole<T>` extractor + `from_extractor` which is preferred.
///
/// When no `AuthContext` is present (auth disabled) access is allowed.
pub async fn require_role_mw(
    State(roles): State<std::sync::Arc<Vec<Role>>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, RoleError> {
    match req
        .extensions()
        .get::<AuthContext>()
        .map(|c| c.claims.role.clone())
    {
        None => Ok(next.run(req).await),
        Some(role) if roles.contains(&role) => Ok(next.run(req).await),
        Some(_) => Err(RoleError::Forbidden),
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::model::{Claims, Role};
    use axum::http::Request;

    fn ctx(role: Role) -> AuthContext {
        AuthContext {
            claims: Claims {
                sub: "user-1".into(),
                tenant_id: "tenant-a".into(),
                role,
                exp: 9999999999,
                iat: 1000000000,
            },
        }
    }

    fn parts_with(role: Role) -> Parts {
        let mut req = Request::new(());
        req.extensions_mut().insert(ctx(role));
        req.into_parts().0
    }

    fn parts_empty() -> Parts {
        Request::new(()).into_parts().0
    }

    #[tokio::test]
    async fn admin_passes_admin_only() {
        let mut p = parts_with(Role::Admin);
        let r = RequireRole::<AuthManageSet>::from_request_parts(&mut p, &()).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn operator_rejected_admin_only() {
        let mut p = parts_with(Role::Operator);
        let r = RequireRole::<AuthManageSet>::from_request_parts(&mut p, &()).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn tenant_user_rejected_admin_only() {
        let mut p = parts_with(Role::TenantUser);
        let r = RequireRole::<AuthManageSet>::from_request_parts(&mut p, &()).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn all_roles_pass_read() {
        for role in ALL_ROLES {
            let mut p = parts_with(role.clone());
            let r = RequireRole::<ReadSet>::from_request_parts(&mut p, &()).await;
            assert!(r.is_ok(), "{role:?} should be allowed to read");
        }
    }

    #[tokio::test]
    async fn write_set_excludes_auditor() {
        let mut p = parts_with(Role::Auditor);
        let r = RequireRole::<WriteSet>::from_request_parts(&mut p, &()).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn tenant_admin_passes_write() {
        let mut p = parts_with(Role::TenantAdmin);
        let r = RequireRole::<WriteSet>::from_request_parts(&mut p, &()).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn tenant_user_passes_create() {
        let mut p = parts_with(Role::TenantUser);
        let r = RequireRole::<CreateSet>::from_request_parts(&mut p, &()).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn auditor_rejected_create() {
        let mut p = parts_with(Role::Auditor);
        let r = RequireRole::<CreateSet>::from_request_parts(&mut p, &()).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn missing_auth_context_allowed() {
        let mut p = parts_empty();
        let r = RequireRole::<AuthManageSet>::from_request_parts(&mut p, &()).await;
        assert!(r.is_ok());
    }

    #[test]
    fn forbidden_response_is_403() {
        let resp = RoleError::Forbidden.into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
