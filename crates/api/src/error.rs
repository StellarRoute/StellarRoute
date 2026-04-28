//! Error types for the API

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use crate::models::{ApiErrorCode, ApiResponse, ErrorResponse};

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
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    #[error("Invalid slippage: {0}")]
    InvalidSlippage(String),
    #[error("Invalid asset format: {0}")]
    InvalidAssetFormat(String),
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
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, ApiErrorCode::BadRequest, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound, msg),
            ApiError::Validation(message) => (
                StatusCode::BAD_REQUEST,
                ApiErrorCode::ValidationError,
                message,
            ),
            ApiError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                ApiErrorCode::RateLimitExceeded,
                "Too many requests. Please try again later.".to_string(),
            ),
            ApiError::Overloaded(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                ApiErrorCode::Overloaded,
                msg,
            ),
            ApiError::Unauthorized(msg) => {
                (StatusCode::UNAUTHORIZED, ApiErrorCode::Unauthorized, msg)
            }
            ApiError::InvalidAsset(msg) => {
                (StatusCode::BAD_REQUEST, ApiErrorCode::InvalidAsset, msg)
            }
            ApiError::InvalidAmount(msg) => {
                (StatusCode::BAD_REQUEST, ApiErrorCode::InvalidAmount, msg)
            }
            ApiError::InvalidSlippage(msg) => {
                (StatusCode::BAD_REQUEST, ApiErrorCode::InvalidSlippage, msg)
            }
            ApiError::InvalidAssetFormat(msg) => (
                StatusCode::BAD_REQUEST,
                ApiErrorCode::InvalidAssetFormat,
                msg,
            ),
            ApiError::NoRouteFound => (
                StatusCode::NOT_FOUND,
                ApiErrorCode::NoRoute,
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
                let payload = ErrorResponse::new(
                    ApiErrorCode::StaleMarketData,
                    "All market data inputs are stale",
                )
                .with_details(details);
                let body = Json(ApiResponse::new(payload, "system"));
                return (StatusCode::UNPROCESSABLE_ENTITY, body).into_response();
            }
            ApiError::Database(_) | ApiError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::InternalError,
                "An internal error occurred".to_string(),
            ),
        };

        let payload = ErrorResponse::new(error_type, message);
        let body = Json(ApiResponse::new(payload, "system"));
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
        let envelope: serde_json::Value = serde_json::from_slice(&body).expect("json");
        (status, envelope["data"].clone())
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

    #[tokio::test]
    async fn bad_request_mapping() {
        let err = ApiError::BadRequest("invalid query".to_string());
        let (status, json) = response_parts(err).await;
        assert_eq!(status, 400);
        assert_eq!(json["error"], "bad_request");
    }

    #[tokio::test]
    async fn not_found_mapping() {
        let err = ApiError::NotFound("pair missing".to_string());
        let (status, json) = response_parts(err).await;
        assert_eq!(status, 404);
        assert_eq!(json["error"], "not_found");
    }

    #[tokio::test]
    async fn validation_mapping() {
        let err = ApiError::Validation("amount low".to_string());
        let (status, json) = response_parts(err).await;
        assert_eq!(status, 400);
        assert_eq!(json["error"], "validation_error");
    }

    #[tokio::test]
    async fn rate_limit_mapping() {
        let err = ApiError::RateLimitExceeded;
        let (status, json) = response_parts(err).await;
        assert_eq!(status, 429);
        assert_eq!(json["error"], "rate_limit_exceeded");
    }

    #[tokio::test]
    async fn internal_error_mapping() {
        let err = ApiError::Internal(Arc::new(anyhow::anyhow!("oops")));
        let (status, json) = response_parts(err).await;
        assert_eq!(status, 500);
        assert_eq!(json["error"], "internal_error");
    }

    #[tokio::test]
    async fn database_error_mapping() {
        // sqlx doesn't allow easy mock error creation, but we can check it maps to internal_error
        let err = ApiError::Database(Arc::new(sqlx::Error::PoolClosed));
        let (status, json) = response_parts(err).await;
        assert_eq!(status, 500);
        assert_eq!(json["error"], "internal_error");
    }
}
