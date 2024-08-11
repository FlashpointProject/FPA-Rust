use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    InternalServerError,
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, err_msg) = match self {
            Self::InternalServerError => (StatusCode::INTERNAL_SERVER_ERROR, "An internal server error occured"),
            Self::NotFound => (StatusCode::NOT_FOUND, "Resource not found")
        };
        (status, Json(json!({ "error": err_msg }))).into_response()
    }
}