//! API response models

use axum::{
    body::{Body, Bytes},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

/// Standard API response envelope
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub v: u8,
    pub timestamp: i64,
    pub request_id: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T, request_id: impl Into<String>) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        Self {
            v: 1,
            timestamp,
            request_id: request_id.into(),
            data,
        }
    }
}

/// Per-component health status value
pub type ComponentStatus = String;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub components: std::collections::HashMap<String, ComponentStatus>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CacheMetricsResponse {
    pub quote_hits: u64,
    pub quote_misses: u64,
    pub stale_quote_rejections: u64,
    pub stale_inputs_excluded: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TradingPair {
    pub base: String,
    pub counter: String,
    pub base_asset: String,
    pub counter_asset: String,
    pub offer_count: i64,
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AssetInfo {
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
}

impl AssetInfo {
    pub fn native() -> Self {
        Self {
            asset_type: "native".to_string(),
            asset_code: None,
            asset_issuer: None,
        }
    }

    pub fn credit(code: String, issuer: Option<String>) -> Self {
        let asset_type = if code.len() <= 4 {
            "credit_alphanum4"
        } else {
            "credit_alphanum12"
        };

        Self {
            asset_type: asset_type.to_string(),
            asset_code: Some(code),
            asset_issuer: issuer,
        }
    }

    pub fn display_name(&self) -> String {
        self.asset_code.clone().unwrap_or_else(|| "XLM".to_string())
    }

    pub fn to_canonical(&self) -> String {
        match (&self.asset_code, &self.asset_issuer) {
            (None, _) => "native".to_string(),
            (Some(code), Some(issuer)) => format!("{}:{}", code, issuer),
            (Some(code), None) => code.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PairsResponse {
    pub pairs: Vec<TradingPair>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderbookResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderbookLevel {
    pub price: String,
    pub amount: String,
    pub total: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct DataFreshness {
    pub fresh_count: usize,
    pub stale_count: usize,
    pub max_staleness_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    pub amount: String,
    pub price: String,
    pub total: String,
    pub quote_type: String,
    pub path: Vec<PathStep>,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<QuoteRationaleMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_impact: Option<String>,

    /// 🔥 NOW FED FROM ROUTING POLICY ENGINE
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusion_diagnostics: Option<ExclusionDiagnostics>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_freshness: Option<DataFreshness>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchQuoteResponse {
    pub quotes: Vec<QuoteResponse>,
    pub total: usize,
    /// Unix timestamp (ms) of the shared market snapshot used for all items.
    /// All quotes in this batch were computed against data no older than this.
    pub snapshot_timestamp: i64,
}

/// Result for a single item in a batch quote response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchQuoteItemResult {
    /// Zero-based index of this item in the original request.
    pub index: usize,
    /// The quote, present when `status == "ok"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<QuoteResponse>,
    /// Per-item error, present when `status == "error"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BatchItemError>,
    /// `"ok"` or `"error"`.
    pub status: String,
}

impl BatchQuoteItemResult {
    pub fn ok(index: usize, quote: QuoteResponse) -> Self {
        Self {
            index,
            quote: Some(quote),
            error: None,
            status: "ok".to_string(),
        }
    }

    pub fn err(index: usize, error: BatchItemError) -> Self {
        Self {
            index,
            quote: None,
            error: Some(error),
            status: "error".to_string(),
        }
    }
}

/// Per-item error detail in a batch response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchItemError {
    /// Machine-readable error code.
    pub code: String,
    /// Human-readable description.
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RouteResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    pub amount: String,
    pub path: Vec<PathStep>,
    pub slippage_bps: u32,
    pub timestamp: i64,
}

/// A comprehensive set of multiple ranked execution routes
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoutesResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    pub amount: String,
    pub routes: Vec<RouteCandidate>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RouteCandidate {
    pub estimated_output: String,
    pub impact_bps: u32,
    pub score: f64,
    pub policy_used: String,
    pub path: Vec<RouteHop>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RouteHop {
    pub from_asset: AssetInfo,
    pub to_asset: AssetInfo,
    pub price: String,
    pub amount_out_of_hop: String,
    pub fee_bps: u32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteStalenessConfig {
    pub max_age_seconds: u32,
    pub reject_stale: bool,
}

impl Default for QuoteStalenessConfig {
    fn default() -> Self {
        Self {
            max_age_seconds: 30,
            reject_stale: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteRationaleMetadata {
    pub strategy: String,
    pub selected_source: String,
    pub compared_venues: Vec<VenueEvaluation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VenueEvaluation {
    pub source: String,
    pub price: String,
    pub available_amount: String,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PathStep {
    pub from_asset: AssetInfo,
    pub to_asset: AssetInfo,
    pub price: String,
    pub source: String,
}

/// 🔥 ROUTING POLICY → API SURFACE EXPOSURE
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExclusionDiagnostics {
    pub excluded_routes: Vec<ExcludedRouteInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExcludedRouteInfo {
    pub route_id: String,
    pub reason: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ApiErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    InternalError,
    BadRequest,
    NotFound,
    ValidationError,
    RateLimitExceeded,
    Overloaded,
    Unauthorized,
    InvalidAsset,
    NoRoute,
    StaleMarketData,
}

impl ApiErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InternalError => "internal_error",
            Self::BadRequest => "bad_request",
            Self::NotFound => "not_found",
            Self::ValidationError => "validation_error",
            Self::RateLimitExceeded => "rate_limit_exceeded",
            Self::Overloaded => "overloaded",
            Self::Unauthorized => "unauthorized",
            Self::InvalidAsset => "invalid_asset",
            Self::InvalidAmount => "invalid_amount",
            Self::InvalidSlippage => "invalid_slippage",
            Self::InvalidAssetFormat => "invalid_asset_format",
            Self::NoRoute => "no_route",
            Self::StaleMarketData => "stale_market_data",
        }
    }
}

impl ErrorResponse {
    pub fn new(error: ApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            error,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}
