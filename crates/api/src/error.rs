use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("Request error: {:?}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({ "error": self.0.to_string() }).to_string(),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
