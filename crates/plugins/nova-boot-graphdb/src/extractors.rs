use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use std::fmt;

/// Nova graph database extractor.
///
/// Provides access to the graph database injected by the `NovaGraphDb` plugin.
#[derive(Clone)]
pub struct NovaGraph(pub crate::NovaGraphDb);

impl std::fmt::Debug for NovaGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NovaGraph").field(&"<graph>").finish()
    }
}

impl<S> FromRequestParts<S> for NovaGraph
where
    S: Send + Sync,
{
    type Rejection = NovaGraphRejection;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = parts
            .extensions
            .get::<crate::NovaGraphDb>()
            .cloned()
            .map(NovaGraph)
            .ok_or(NovaGraphRejection);

        async move { result }
    }
}

#[derive(Debug)]
pub struct NovaGraphRejection;

impl fmt::Display for NovaGraphRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Graph database not found in request extensions. Did you forget to add the NovaGraphDb plugin to NovaApp?"
        )
    }
}

impl std::error::Error for NovaGraphRejection {}

impl IntoResponse for NovaGraphRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Graph database not configured",
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
    async fn extracts_graph_from_extensions() {
        let graph = crate::NovaGraphDb::in_memory();
        let (mut parts, _) = Request::new(()).into_parts();
        parts.extensions.insert(graph);

        let extracted = NovaGraph::from_request_parts(&mut parts, &()).await;

        assert!(extracted.is_ok());
    }

    #[tokio::test]
    async fn rejects_when_graph_is_missing() {
        let (mut parts, _) = Request::new(()).into_parts();

        let rejection = NovaGraph::from_request_parts(&mut parts, &())
            .await
            .expect_err("expected missing graph rejection");

        assert!(rejection.to_string().contains("NovaGraphDb"));
        assert_eq!(
            rejection.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
