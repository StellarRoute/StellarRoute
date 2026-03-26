//! Error types for the API

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use crate::models::ErrorResponse;

use std::sync::Arc;

#[derive(Error, Debug, Clone)]
pub enum ApiError {
    #[error("Internal server error: {0}")]
    Internal(Arc<anyhow::Error>),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(Arc<sqlx::Error>),

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

    #[error("All market data inputs are stale ({stale_count} stale, {fresh_count} fresh)")]
    StaleMarketData {
        stale_count: usize,
        fresh_count: usize,
        threshold_secs_sdex: u64,
        threshold_secs_amm: u64,
    },
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(Arc::new(err))
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(Arc::new(err))
    }
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
            ApiError::Overloaded(msg) => (StatusCode::SERVICE_UNAVAILABLE, "overloaded", msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg),
            ApiError::InvalidAsset(msg) => (StatusCode::BAD_REQUEST, "invalid_asset", msg),
            ApiError::NoRouteFound => (
                StatusCode::NOT_FOUND,
                "no_route",
                "No trading route found for this pair".to_string(),
            ),
            ApiError::StaleMarketData {
                stale_count,
                fresh_count,
                threshold_secs_sdex,
                threshold_secs_amm,
            } => {
                let details = serde_json::json!({
                    "stale_count": stale_count,
                    "fresh_count": fresh_count,
                    "threshold_secs_sdex": threshold_secs_sdex,
                    "threshold_secs_amm": threshold_secs_amm,
                });
                let body = Json(
                    ErrorResponse::new("stale_market_data", "All market data inputs are stale")
                        .with_details(details),
                );
                return (StatusCode::UNPROCESSABLE_ENTITY, body).into_response();
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::response::IntoResponse;

    async fn response_parts(err: ApiError) -> (u16, serde_json::Value) {
        let response = err.into_response();
        let status = response.status().as_u16();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        (status, json)
    }

    #[tokio::test]
    async fn stale_market_data_returns_422() {
        let err = ApiError::StaleMarketData {
            stale_count: 3,
            fresh_count: 0,
            threshold_secs_sdex: 30,
            threshold_secs_amm: 60,
        };
        let (status, _) = response_parts(err).await;
        assert_eq!(status, 422);
    }

    #[tokio::test]
    async fn stale_market_data_error_field() {
        let err = ApiError::StaleMarketData {
            stale_count: 2,
            fresh_count: 0,
            threshold_secs_sdex: 30,
            threshold_secs_amm: 60,
        };
        let (_, json) = response_parts(err).await;
        assert_eq!(json["error"], "stale_market_data");
    }

    #[tokio::test]
    async fn stale_market_data_details_fields() {
        let err = ApiError::StaleMarketData {
            stale_count: 5,
            fresh_count: 1,
            threshold_secs_sdex: 30,
            threshold_secs_amm: 60,
        };
        let (_, json) = response_parts(err).await;
        let details = &json["details"];
        assert_eq!(details["stale_count"], 5);
        assert_eq!(details["fresh_count"], 1);
        assert_eq!(details["threshold_secs_sdex"], 30);
        assert_eq!(details["threshold_secs_amm"], 60);
    }
}
