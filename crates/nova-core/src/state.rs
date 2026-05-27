use axum::{
    extract::FromRequestParts,
    http::StatusCode,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use std::fmt;

#[derive(Debug, Clone)]
pub struct NovaState<S>(pub S);

impl<S, State> FromRequestParts<State> for NovaState<S>
where
    S: Clone + Send + Sync + 'static,
    State: Send + Sync,
{
    type Rejection = NovaStateRejection;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &State,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = parts
            .extensions
            .get::<S>()
            .cloned()
            .map(NovaState)
            .ok_or(NovaStateRejection);

        async move { result }
    }
}

/// Error returned when application state is missing.
#[derive(Debug)]
pub struct NovaStateRejection;

impl fmt::Display for NovaStateRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Application state is not present in request extensions. \
                   Ensure you added the state via `NovaApp::new()` and did not remove the `Extension` layer."
        )
    }
}

impl std::error::Error for NovaStateRejection {}

impl IntoResponse for NovaStateRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error: application state missing",
        )
            .into_response()
    }
}
