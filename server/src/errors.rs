use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn error_response(status: StatusCode, msg: &str) -> Response {
    (status, Json(ErrorResponse { error: msg.into() })).into_response()
}