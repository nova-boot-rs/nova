use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use std::fmt;

/// Nova message bus extractor.
///
/// Provides access to the message broker injected by the `NovaMessaging` plugin.
#[derive(Clone)]
pub struct NovaBus(pub crate::NovaMessaging);

impl std::fmt::Debug for NovaBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NovaBus").field(&"<bus>").finish()
    }
}

impl<S> FromRequestParts<S> for NovaBus
where
    S: Send + Sync,
{
    type Rejection = NovaBusRejection;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = parts
            .extensions
            .get::<crate::NovaMessaging>()
            .cloned()
            .map(NovaBus)
            .ok_or(NovaBusRejection);

        async move { result }
    }
}

#[derive(Debug)]
pub struct NovaBusRejection;

impl fmt::Display for NovaBusRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Message bus not found in request extensions. Did you forget to add the NovaMessaging plugin to NovaApp?"
        )
    }
}

impl std::error::Error for NovaBusRejection {}

impl IntoResponse for NovaBusRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Message bus not configured",
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
    async fn extracts_bus_from_extensions() {
        let bus = crate::NovaMessaging::in_memory();
        let (mut parts, _) = Request::new(()).into_parts();
        parts.extensions.insert(bus);

        let extracted = NovaBus::from_request_parts(&mut parts, &()).await;

        assert!(extracted.is_ok());
    }

    #[tokio::test]
    async fn rejects_when_bus_is_missing() {
        let (mut parts, _) = Request::new(()).into_parts();

        let rejection = NovaBus::from_request_parts(&mut parts, &())
            .await
            .expect_err("expected missing bus rejection");

        assert!(rejection.to_string().contains("NovaMessaging"));
        assert_eq!(
            rejection.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
