//! Strongly-typed request and response models for the StellarRoute API.
//!
//! All types derive `Serialize`/`Deserialize` and map 1-to-1 with the
//! OpenAPI schema in `docs/api/openapi.yaml`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Health ────────────────────────────────────────────────────────────────────

/// Response from `GET /health`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall service status: `"healthy"` or `"unhealthy"`.
    pub status: String,
    /// ISO-8601 UTC timestamp of the health check.
    pub timestamp: String,
    /// Deployed crate version string.
    pub version: String,
    /// Per-dependency health map, e.g. `{"database": "healthy"}`.
    pub components: HashMap<String, String>,
}

impl HealthResponse {
    /// Returns `true` when `status == "healthy"`.
    pub fn is_healthy(&self) -> bool {
        self.status == "healthy"
    }
}

// ── Assets ────────────────────────────────────────────────────────────────────

/// Stellar asset descriptor returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetInfo {
    /// Stellar asset type: `"native"`, `"credit_alphanum4"`, or `"credit_alphanum12"`.
    pub asset_type: String,
    /// Asset code, e.g. `"USDC"`. `None` for native XLM.
    pub asset_code: Option<String>,
    /// G-address of the issuing account. `None` for native XLM.
    pub asset_issuer: Option<String>,
}

impl AssetInfo {
    /// Returns a human-readable identifier: `"native"`, `"CODE"`, or `"CODE:ISSUER"`.
    pub fn display_name(&self) -> String {
        match (&self.asset_code, &self.asset_issuer) {
            (Some(code), Some(issuer)) => format!("{code}:{issuer}"),
            (Some(code), None) => code.clone(),
            _ => "native".to_string(),
        }
    }

    /// Returns `true` if this is the native XLM asset.
    pub fn is_native(&self) -> bool {
        self.asset_type == "native"
    }
}

// ── Trading pairs ─────────────────────────────────────────────────────────────

/// A single tradeable asset pair with active orderbook depth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingPair {
    /// Human-readable base asset code, e.g. `"XLM"`.
    pub base: String,
    /// Human-readable counter asset code, e.g. `"USDC"`.
    pub counter: String,
    /// Canonical base asset identifier (`"native"` or `"CODE:ISSUER"`).
    pub base_asset: String,
    /// Canonical counter asset identifier.
    pub counter_asset: String,
    /// Number of active offers for this pair.
    pub offer_count: i64,
    /// RFC-3339 timestamp of the most recent offer update.
    pub last_updated: Option<String>,
}

/// Response from `GET /api/v1/pairs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairsResponse {
    /// Active trading pairs ordered by liquidity depth.
    pub pairs: Vec<TradingPair>,
    /// Total number of pairs returned.
    pub total: usize,
}

// ── Orderbook ─────────────────────────────────────────────────────────────────

/// A single price level in the orderbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookLevel {
    /// Price as a decimal string (7 decimal places).
    pub price: String,
    /// Available amount at this price level.
    pub amount: String,
    /// Total value at this price level (`price × amount`).
    pub total: String,
}

/// Summary information for an orderbook snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookSummary {
    pub bid: Option<String>,
    pub ask: Option<String>,
    pub spread_bps: Option<i64>,
    pub midpoint: Option<String>,
}

/// Response from `GET /api/v1/orderbook/{base}/{quote}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    /// Buy orders sorted highest price first.
    pub bids: Vec<OrderbookLevel>,
    /// Sell orders sorted lowest price first.
    pub asks: Vec<OrderbookLevel>,
    /// Snapshot summary (best bid/ask, midpoint, spread in bps).
    pub summary: OrderbookSummary,
    /// Unix timestamp of the snapshot.
    pub timestamp: i64,
}

impl OrderbookResponse {
    /// Returns the best bid price (highest buy offer), if any.
    pub fn best_bid(&self) -> Option<&str> {
        self.bids.first().map(|l| l.price.as_str())
    }

    /// Returns the best ask price (lowest sell offer), if any.
    pub fn best_ask(&self) -> Option<&str> {
        self.asks.first().map(|l| l.price.as_str())
    }
}

// ── Quote ─────────────────────────────────────────────────────────────────────

/// A single hop in the optimal execution path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathStep {
    pub from_asset: AssetInfo,
    pub to_asset: AssetInfo,
    /// Exchange rate for this hop.
    pub price: String,
    /// Liquidity source: `"sdex"` or `"amm:<pool_address>"`.
    pub source: String,
}

/// Direction of a price quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuoteType {
    /// How much quote asset you receive when selling `amount` of the base asset.
    Sell,
    /// How much base asset you must spend to buy `amount` of the quote asset.
    Buy,
}

impl QuoteType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sell => "sell",
            Self::Buy => "buy",
        }
    }
}

impl std::fmt::Display for QuoteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Response from `GET /api/v1/quote/{base}/{quote}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    /// Input amount that was quoted.
    pub amount: String,
    /// Effective price (quote asset per base asset unit).
    pub price: String,
    /// Total output amount (`amount × price`).
    pub total: String,
    /// Direction of the quote.
    pub quote_type: String,
    /// Ordered list of hops in the optimal execution path.
    pub path: Vec<PathStep>,
    /// Unix timestamp when the quote was generated.
    pub timestamp: i64,
}

/// Response from `POST /api/v1/batch/quote`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchQuoteResponse {
    /// Array of quotes in the same order as requested.
    pub quotes: Vec<QuoteResponse>,
    /// Total number of quotes successfully fetched.
    pub total: usize,
}

// ── Routes ───────────────────────────────────────────────────────────────────

/// Request parameters for `GET /api/v1/routes/{base}/{quote}`.
#[derive(Debug, Clone)]
pub struct RoutesRequest<'a> {
    /// Base asset identifier (path parameter).
    pub base: &'a str,
    /// Quote asset identifier (path parameter).
    pub quote: &'a str,
    /// Amount to route, expressed in the base asset's atomic units.
    pub amount: u64,
    /// Maximum acceptable slippage in basis points.
    pub slippage_bps: Option<u16>,
    /// Quote type filter.
    pub quote_type: Option<QuoteType>,
}

/// Response from `GET /api/v1/routes/{base}/{quote}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutesResponse {
    /// Optional base asset metadata included by the API.
    #[serde(default)]
    pub base_asset: Option<AssetInfo>,
    /// Optional quote asset metadata included by the API.
    #[serde(default)]
    pub quote_asset: Option<AssetInfo>,
    /// Amount used to compute the route candidates.
    #[serde(default)]
    pub amount: String,
    /// Ranked route candidates returned by the endpoint.
    pub routes: Vec<Route>,
    /// Unix timestamp when the route computation completed.
    #[serde(default)]
    pub timestamp: i64,
}

/// A single ranked route candidate returned by the routes endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Route {
    /// Estimated output amount for the full route.
    #[serde(default)]
    pub estimated_output: String,
    /// Estimated price impact in basis points.
    #[serde(default)]
    pub impact_bps: u32,
    /// Composite quality score for the route candidate.
    #[serde(default)]
    pub score: f64,
    /// Optimizer policy used to produce the route.
    #[serde(default)]
    pub policy_used: String,
    /// Ordered list of execution hops that make up the route.
    #[serde(default)]
    pub path: Vec<RouteHop>,
}

/// A single hop within a ranked route candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteHop {
    /// Asset transferred from at this hop.
    #[serde(default)]
    pub from_asset: Option<AssetInfo>,
    /// Asset transferred to at this hop.
    #[serde(default)]
    pub to_asset: Option<AssetInfo>,
    /// Exchange rate for this hop.
    #[serde(default)]
    pub price: String,
    /// Fee charged by the source for this hop, in basis points.
    #[serde(default)]
    pub fee_bps: Option<u32>,
    /// Amount produced after this hop.
    #[serde(default)]
    pub amount_out_of_hop: String,
    /// Liquidity source for this hop.
    #[serde(default)]
    pub source: String,
}

// ── Request types ─────────────────────────────────────────────────────────────

/// Parameters for `GET /api/v1/quote/{base}/{quote}`.
#[derive(Debug, Clone)]
pub struct QuoteRequest<'a> {
    /// Base asset identifier: `"native"`, `"CODE"`, or `"CODE:ISSUER"`.
    pub base: &'a str,
    /// Quote asset identifier.
    pub quote: &'a str,
    /// Amount of the base asset to trade. Defaults to `"1"` when `None`.
    pub amount: Option<&'a str>,
    /// Direction of the quote.
    pub quote_type: QuoteType,
}

impl<'a> QuoteRequest<'a> {
    /// Convenience constructor for a sell quote with no explicit amount.
    pub fn sell(base: &'a str, quote: &'a str) -> Self {
        Self {
            base,
            quote,
            amount: None,
            quote_type: QuoteType::Sell,
        }
    }

    /// Convenience constructor for a buy quote with no explicit amount.
    pub fn buy(base: &'a str, quote: &'a str) -> Self {
        Self {
            base,
            quote,
            amount: None,
            quote_type: QuoteType::Buy,
        }
    }
}

/// A request item for a batch quote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRequestItem {
    pub base: String,
    pub quote: String,
    pub amount: Option<String>,
    pub slippage_bps: Option<u32>,
    pub quote_type: Option<QuoteType>,
}

/// Parameters for `POST /api/v1/batch/quote`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchQuoteRequest {
    /// Array of quote requests to fetch.
    pub quotes: Vec<QuoteRequestItem>,
}

// ── Internal error response ───────────────────────────────────────────────────

/// Wire format of the API error body — used internally by the client.
#[derive(Debug, Deserialize)]
pub(crate) struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[allow(dead_code)]
    pub details: Option<serde_json::Value>,
}
