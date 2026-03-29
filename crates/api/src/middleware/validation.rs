//! Validation middleware and extractors

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, Query},
    http::request::Parts,
};

use crate::{
    error::ApiError,
    models::{AssetPath, QuoteParams},
};

/// Validated quote request extractor
///
/// Combines path and query parameter parsing with early validation
pub struct ValidatedQuoteRequest {
    pub base: AssetPath,
    pub quote: AssetPath,
    pub params: QuoteParams,
}

#[async_trait]
impl<S> FromRequestParts<S> for ValidatedQuoteRequest
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // 1. Extract path parameters
        let Path((base_str, quote_str)): Path<(String, String)> =
            Path::from_request_parts(parts, state)
                .await
                .map_err(|e| ApiError::BadRequest(format!("Invalid path parameters: {}", e)))?;

        // 2. Extract query parameters
        let Query(params): Query<QuoteParams> = Query::from_request_parts(parts, state)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Invalid query parameters: {}", e)))?;

        // 3. Parse and validate assets
        let base = AssetPath::parse(&base_str).map_err(|message| {
            ApiError::Validation(format!("Invalid base asset: {}", message))
        })?;

        let quote = AssetPath::parse(&quote_str).map_err(|message| {
            ApiError::Validation(format!("Invalid quote asset: {}", message))
        })?;

        // 4. Validate query params (amount, slippage)
        params
            .validate()
            .map_err(|(_code, message)| ApiError::Validation(message))?;

        Ok(ValidatedQuoteRequest {
            base,
            quote,
            params,
        })
    }
}
