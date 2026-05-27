use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use std::fmt;

/// Nova database pool extractor.
///
/// Provides access to the SQL read/write pool injected by the `NovaSql` plugin.
#[derive(Clone)]
pub struct NovaDb(pub crate::ReadWritePool);

impl std::fmt::Debug for NovaDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NovaDb").field(&"<pool>").finish()
    }
}

impl<S> FromRequestParts<S> for NovaDb
where
    S: Send + Sync,
{
    type Rejection = NovaDbRejection;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = parts
            .extensions
            .get::<crate::ReadWritePool>()
            .cloned()
            .map(NovaDb)
            .ok_or(NovaDbRejection);

        async move { result }
    }
}

#[derive(Debug)]
pub struct NovaDbRejection;

impl fmt::Display for NovaDbRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Database pool not found in request extensions. Did you forget to add the NovaSql plugin to NovaApp?"
        )
    }
}

impl std::error::Error for NovaDbRejection {}

impl IntoResponse for NovaDbRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database connection pool not configured",
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    #[tokio::test]
    async fn extracts_pool_from_extensions() {
        let sql = crate::NovaSql::connect("sqlite::memory:", false).await;
        let pool = sql.read_write_pool();
        let (mut parts, _) = Request::new(()).into_parts();
        parts.extensions.insert(pool);

        let extracted = NovaDb::from_request_parts(&mut parts, &()).await;

        assert!(extracted.is_ok());
    }

    #[tokio::test]
    async fn rejects_when_pool_is_missing() {
        let (mut parts, _) = Request::new(()).into_parts();

        let rejection = NovaDb::from_request_parts(&mut parts, &())
            .await
            .expect_err("expected missing pool rejection");

        assert!(rejection.to_string().contains("NovaSql"));
        assert_eq!(
            rejection.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
