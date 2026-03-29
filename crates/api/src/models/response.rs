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

/// Cache metrics response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CacheMetricsResponse {
    pub quote_hits: u64,
    pub quote_misses: u64,
    /// Total quote requests rejected because all inputs were stale
    pub stale_quote_rejections: u64,
    /// Total stale inputs excluded across all successful quotes
    pub stale_inputs_excluded: u64,
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

/// Freshness metadata about the data sources used to compute a quote
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct DataFreshness {
    /// Number of fresh candidates used to compute the quote
    pub fresh_count: usize,
    /// Number of stale candidates excluded from the quote (zero when all are fresh)
    pub stale_count: usize,
    /// Maximum observed staleness in seconds among all evaluated candidates
    pub max_staleness_secs: u64,
}

/// Price quote response with expiry and staleness metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    pub amount: String,
    pub price: String,
    pub total: String,
    pub quote_type: String,
    pub path: Vec<PathStep>,
    /// Unix timestamp (ms) when this quote was generated
    pub timestamp: i64,
    /// Unix timestamp (ms) when this quote expires and should be considered stale
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// Unix timestamp (ms) of the underlying data source (e.g., orderbook snapshot)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_timestamp: Option<i64>,
    /// Time-to-live in seconds for client-side staleness detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u32>,
    /// Rationale for quote venue selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<QuoteRationaleMetadata>,
    /// Estimated price impact percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_impact: Option<String>,
    /// Venues excluded from routing and the reason for each exclusion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusion_diagnostics: Option<ExclusionDiagnostics>,
    /// Freshness metadata about the data sources used to compute this quote
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_freshness: Option<DataFreshness>,
}

/// Response for a batch quote request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchQuoteResponse {
    /// Array of quotes in the same order as requested
    pub quotes: Vec<QuoteResponse>,
    /// Total number of quotes successfully fetched
    pub total: usize,
}

/// Trading route response (path only, no pricing)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RouteResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    pub amount: String,
    pub path: Vec<PathStep>,
    pub slippage_bps: u32,
    /// Unix timestamp (ms) when this route was generated
    pub timestamp: i64,
}
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

/// A single proposed N-hop route with pricing metrics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RouteCandidate {
    pub estimated_output: String,
    pub impact_bps: u32,
    pub score: f64,
    pub policy_used: String,
    pub path: Vec<RouteHop>,
}

/// A specific swap execution step inside a RouteCandidate
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RouteHop {
    pub from_asset: AssetInfo,
    pub to_asset: AssetInfo,
    pub price: String,
    pub amount_out_of_hop: String,
    pub fee_bps: u32,
    pub source: String,
}

/// Configuration for quote staleness detection
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteStalenessConfig {
    /// Maximum quote age in seconds before considering stale
    pub max_age_seconds: u32,
    /// Whether to reject stale quotes on the client side
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

impl QuoteResponse {
    /// Check if this quote is considered stale based on the given config
    pub fn is_stale(&self, config: &QuoteStalenessConfig) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let age_ms = now - self.timestamp;
        let max_age_ms = config.max_age_seconds as i64 * 1000;

        age_ms > max_age_ms
    }

    /// Create a quote response with expiry metadata
    pub fn with_expiry(mut self, ttl_seconds: u32) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        self.expires_at = Some(now + (ttl_seconds as i64 * 1000));
        self.ttl_seconds = Some(ttl_seconds);
        self
    }
}

/// Rationale metadata for quote venue selection
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteRationaleMetadata {
    pub strategy: String,
    pub selected_source: String,
    pub compared_venues: Vec<VenueEvaluation>,
}

/// Per-venue comparison details for direct route evaluation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VenueEvaluation {
    pub source: String,
    pub price: String,
    pub available_amount: String,
    pub executable: bool,
}

/// Step in a trading path
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PathStep {
    pub from_asset: AssetInfo,
    pub to_asset: AssetInfo,
    pub price: String,
    pub source: String, // "sdex" or "amm:{pool_address}"
}

// ---------------------------------------------------------------------------
// Exclusion diagnostics (local API types — routing types lack ToSchema)
// ---------------------------------------------------------------------------

/// Diagnostics about venues excluded from routing
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExclusionDiagnostics {
    pub excluded_venues: Vec<ExcludedVenueInfo>,
}

/// Details about a single excluded venue
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExcludedVenueInfo {
    pub venue_ref: String,
    pub reason: ExclusionReason,
}

/// Reason a venue was excluded from routing
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ExclusionReason {
    PolicyThreshold { threshold: f64 },
    Override,
    StaleData,
    CircuitBreakerOpen,
}

/// Machine-readable error codes for API failures
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    /// Unexpected server-side failure
    InternalError,
    /// Malformed request or invalid parameters
    BadRequest,
    /// Requested resource not found
    NotFound,
    /// Request parameters failed validation
    ValidationError,
    /// Client exceeded rate limits
    RateLimitExceeded,
    /// Server is temporarily overloaded
    Overloaded,
    /// Request lacks valid credentials
    Unauthorized,
    /// Invalid Stellar asset identifier
    InvalidAsset,
    /// No executable trading route found
    NoRoute,
    /// Underlying market data is too stale to provide a quote
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
            Self::NoRoute => "no_route",
            Self::StaleMarketData => "stale_market_data",
        }
    }
}

/// Error response
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ApiErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
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

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Property test: Round-trip serialization of QuoteResponse with
    // data_freshness (Property 2 — Validates: Requirements 7.2)
    // -----------------------------------------------------------------------

    use proptest::prelude::*;

    prop_compose! {
        fn arb_data_freshness()(
            fresh_count in 0usize..100,
            stale_count in 0usize..100,
            max_staleness_secs in 0u64..3600,
        ) -> DataFreshness {
            DataFreshness { fresh_count, stale_count, max_staleness_secs }
        }
    }

    proptest! {
        /// **Property 2: Round-trip — serialize then deserialize any `QuoteResponse`
        /// with a `data_freshness` field produces a value equal to the original.**
        ///
        /// **Validates: Requirements 7.2**
        #[test]
        fn data_freshness_round_trip(df in arb_data_freshness()) {
            let serialized = serde_json::to_string(&df).expect("serialize");
            let deserialized: DataFreshness = serde_json::from_str(&serialized).expect("deserialize");
            prop_assert_eq!(df, deserialized);
        }
    }

    // -----------------------------------------------------------------------
    // Unit tests for DataFreshness serialization edge cases (Task 6.2)
    // -----------------------------------------------------------------------

    /// Req 7.3: Unknown fields inside `data_freshness` are ignored on deserialization.
    #[test]
    fn unknown_fields_in_data_freshness_are_ignored() {
        let json = r#"{"fresh_count":3,"stale_count":1,"max_staleness_secs":45,"unknown_field":"ignored"}"#;
        let df: DataFreshness =
            serde_json::from_str(json).expect("should deserialize without error");
        assert_eq!(df.fresh_count, 3);
        assert_eq!(df.stale_count, 1);
        assert_eq!(df.max_staleness_secs, 45);
    }

    /// Req 7.4: Missing `data_freshness` field in QuoteResponse deserializes to None.
    #[test]
    fn missing_data_freshness_deserializes_to_none() {
        let json = r#"{
            "base_asset": {"asset_type": "native"},
            "quote_asset": {"asset_type": "native"},
            "amount": "1.0000000",
            "price": "1.0000000",
            "total": "1.0000000",
            "quote_type": "sell",
            "path": [],
            "timestamp": 1700000000000
        }"#;
        let qr: QuoteResponse =
            serde_json::from_str(json).expect("should deserialize without error");
        assert!(qr.data_freshness.is_none());
    }

    /// Req 3.3: stale_count is zero when all candidates are fresh.
    #[test]
    fn stale_count_zero_when_all_fresh() {
        let df = DataFreshness {
            fresh_count: 5,
            stale_count: 0,
            max_staleness_secs: 10,
        };
        let serialized = serde_json::to_string(&df).expect("serialize");
        let deserialized: DataFreshness = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized.stale_count, 0);
    }

    /// Req 3.4: DataFreshness serializes with snake_case field names.
    #[test]
    fn data_freshness_uses_snake_case_field_names() {
        let df = DataFreshness {
            fresh_count: 2,
            stale_count: 1,
            max_staleness_secs: 30,
        };
        let json = serde_json::to_value(&df).expect("serialize");
        assert!(
            json.get("fresh_count").is_some(),
            "fresh_count key must exist"
        );
        assert!(
            json.get("stale_count").is_some(),
            "stale_count key must exist"
        );
        assert!(
            json.get("max_staleness_secs").is_some(),
            "max_staleness_secs key must exist"
        );
        // Ensure no camelCase variants leaked
        assert!(json.get("freshCount").is_none());
        assert!(json.get("staleCount").is_none());
        assert!(json.get("maxStalenessSecs").is_none());
    }
}
