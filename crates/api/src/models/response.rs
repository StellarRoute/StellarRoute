//! API response models

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Per-component health status value
pub type ComponentStatus = String;

/// Health check response — matches GET /health spec
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Overall service status: "healthy" or "unhealthy"
    pub status: String,
    /// ISO-8601 UTC timestamp of this check
    pub timestamp: String,
    /// Crate version
    pub version: String,
    /// Per-dependency status ("healthy" | "unhealthy")
    pub components: std::collections::HashMap<String, ComponentStatus>,
}

/// Trading pair information — matches GET /api/v1/pairs spec
///
/// `base` / `counter` are human-readable codes (e.g. "XLM", "USDC").
/// `base_asset` / `counter_asset` are canonical Stellar asset identifiers
/// ("native" for XLM, or "CODE:ISSUER" for issued assets).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TradingPair {
    /// Human-readable base asset code (e.g. "XLM")
    pub base: String,
    /// Human-readable counter asset code (e.g. "USDC")
    pub counter: String,
    /// Canonical base asset identifier ("native" or "CODE:ISSUER")
    pub base_asset: String,
    /// Canonical counter asset identifier ("native" or "CODE:ISSUER")
    pub counter_asset: String,
    /// Number of open offers for this pair
    pub offer_count: i64,
    /// RFC-3339 timestamp of the most recent offer update
    pub last_updated: Option<String>,
}

/// Asset information
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AssetInfo {
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
}

impl AssetInfo {
    /// Create a native XLM asset
    pub fn native() -> Self {
        Self {
            asset_type: "native".to_string(),
            asset_code: None,
            asset_issuer: None,
        }
    }

    /// Create a credit asset
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

    /// Human-readable code ("XLM" for native assets)
    pub fn display_name(&self) -> String {
        match &self.asset_code {
            Some(code) => code.clone(),
            None => "XLM".to_string(),
        }
    }

    /// Canonical Stellar asset identifier: "native" or "CODE:ISSUER"
    pub fn to_canonical(&self) -> String {
        match (&self.asset_code, &self.asset_issuer) {
            (None, _) => "native".to_string(),
            (Some(code), Some(issuer)) => format!("{}:{}", code, issuer),
            (Some(code), None) => code.clone(),
        }
    }
}

/// List of trading pairs
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PairsResponse {
    pub pairs: Vec<TradingPair>,
    pub total: usize,
}

/// Orderbook response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderbookResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
    pub timestamp: i64,
}

/// Orderbook price level
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderbookLevel {
    pub price: String,
    pub amount: String,
    pub total: String,
}

/// Price quote response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuoteResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    pub amount: String,
    pub price: String,
    pub total: String,
    pub quote_type: String,
    pub path: Vec<PathStep>,
    pub timestamp: i64,
}

/// Step in a trading path
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PathStep {
    pub from_asset: AssetInfo,
    pub to_asset: AssetInfo,
    pub price: String,
    pub source: String, // "sdex" or "amm:{pool_address}"
}

/// Error response
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}
