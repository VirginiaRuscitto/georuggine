use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct ErrorPayload {
    pub error: String,
}
 
impl ErrorPayload {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}
 
pub fn error_response(status: StatusCode, msg: &str) -> Response {
    (status, Json(ErrorPayload::new(msg))).into_response()
}