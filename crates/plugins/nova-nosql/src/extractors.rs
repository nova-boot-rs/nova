use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use std::fmt;

/// Nova document store extractor.
///
/// Provides access to the NoSQL document store injected by the `NovaNoSql` plugin.
#[derive(Clone)]
pub struct NovaDocs(pub crate::NovaNoSql);

impl std::fmt::Debug for NovaDocs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NovaDocs").field(&"<store>").finish()
    }
}

impl<S> FromRequestParts<S> for NovaDocs
where
    S: Send + Sync,
{
    type Rejection = NovaDocsRejection;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = parts
            .extensions
            .get::<crate::NovaNoSql>()
            .cloned()
            .map(NovaDocs)
            .ok_or(NovaDocsRejection);

        async move { result }
    }
}

#[derive(Debug)]
pub struct NovaDocsRejection;

impl fmt::Display for NovaDocsRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Document store not found in request extensions. Did you forget to add the NovaNoSql plugin to NovaApp?"
        )
    }
}

impl std::error::Error for NovaDocsRejection {}

impl IntoResponse for NovaDocsRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Document store not configured",
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::FromRequestParts;
    use axum::http::Request;
    use std::sync::Arc;

    #[tokio::test]
    async fn extracts_store_from_extensions() {
        let store =
            crate::NovaNoSql::new(Arc::new(crate::memory::InMemoryDocumentStore::default()));
        let (mut parts, _) = Request::new(()).into_parts();
        parts.extensions.insert(store);

        let extracted = NovaDocs::from_request_parts(&mut parts, &()).await;

        assert!(extracted.is_ok());
    }

    #[tokio::test]
    async fn rejects_when_store_is_missing() {
        let (mut parts, _) = Request::new(()).into_parts();

        let rejection = NovaDocs::from_request_parts(&mut parts, &())
            .await
            .expect_err("expected missing store rejection");

        assert!(rejection.to_string().contains("NovaNoSql"));
        assert_eq!(
            rejection.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
