use super::tenant::{Tenant, TenantResolver};
use axum::{
    extract::{Request, FromRequestParts},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// Axum middleware that resolves tenant from the request.
pub async fn tenant_middleware(
    axum::extract::State(resolver): axum::extract::State<Arc<dyn TenantResolver>>,
    mut request: Request,
    next: Next,
) -> Response {
    let headers = request.headers().clone();
    let path = request.uri().path().to_string();
    
    if let Some(tenant) = resolver.resolve(&headers, &path).await {
        request.extensions_mut().insert(tenant);
    }
    next.run(request).await
}

/// Convenience extractor for handlers.
#[derive(Debug, Clone)]
pub struct CurrentTenant(pub Tenant);

#[async_trait::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for CurrentTenant {
    type Rejection = (axum::http::StatusCode, &'static str);

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let tenant = parts
            .extensions
            .get::<Tenant>()
            .cloned()
            .map(CurrentTenant);
        
        async move {
            tenant.ok_or((axum::http::StatusCode::UNAUTHORIZED, "Tenant not found"))
        }
    }
}
