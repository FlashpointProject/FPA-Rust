use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    InternalServerError,
    NotFound,
    AuthError(String),
    Forbidden,
    Unauthorized,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, err_msg) = match self {
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal server error occured".to_owned(),
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_owned()),
            Self::AuthError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Auth error: {}", &msg),
            ),
            Self::Forbidden => (StatusCode::FORBIDDEN, "Access denied".to_owned()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()),
        };
        (status, Json(json!({ "error": err_msg }))).into_response()
    }
}
