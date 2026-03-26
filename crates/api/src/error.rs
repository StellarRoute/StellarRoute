//! Error types for the API

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use crate::models::ErrorResponse;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("System overloaded: {0}")]
    Overloaded(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Invalid asset: {0}")]
    InvalidAsset(String),

    #[error("No route found for trading pair")]
    NoRouteFound,
}

pub type Result<T> = std::result::Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            ApiError::Validation(msg) => (StatusCode::BAD_REQUEST, "validation_error", msg),
            ApiError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limit_exceeded",
                "Too many requests. Please try again later.".to_string(),
            ),
            ApiError::Overloaded(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "overloaded",
                msg,
            ),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg),
            ApiError::InvalidAsset(msg) => (StatusCode::BAD_REQUEST, "invalid_asset", msg),
            ApiError::NoRouteFound => (
                StatusCode::NOT_FOUND,
                "no_route",
                "No trading route found for this pair".to_string(),
            ),
            ApiError::Database(_) | ApiError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "An internal error occurred".to_string(),
            ),
        };

        let body = Json(ErrorResponse::new(error_type, message));
        (status, body).into_response()
    }
}
