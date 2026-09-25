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

/// Chain-scoped asset used by the `/api/v2` seam.
///
/// Wire form is CAIP-inspired. Solana/TRON use internal network labels (not
/// genesis-hash CAIP-2). Natives use numeric SLIP-44 (never `slip44:native`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainAsset {
    /// Chain id, e.g. `"stellar:pubnet"`, `"eip155:1"`.
    pub chain_id: String,
    /// Asset suffix, e.g. `"slip44:148"`, `"erc20:0x…"`.
    pub asset: String,
    /// Full canonical id (`{chain_id}/{asset}`).
    pub canonical: String,
    /// Optional human symbol (not unique across chains).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// Bridge / cross-chain venue metadata (abstraction only — not executable).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeVenueMeta {
    pub provider: String,
    pub source_chain: String,
    pub destination_chain: String,
}

/// Response from `GET /api/v2` (inside the standard `{ v, data, … }` envelope).
///
/// `/api/v2` seam surface today: this info descriptor and
/// `POST /api/v2/assets/canonicalize` only — there is no v2 quote endpoint.
/// Prefer calling those HTTP paths directly; DTO types here stay wire-aligned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiV2Info {
    pub version: u8,
    pub chain_aware_assets: bool,
    pub bridge_venues_metadata_only: bool,
    /// Always `false` until bridge settlement exists.
    #[serde(default)]
    pub bridge_settlement_executable: bool,
    pub supported_chain_namespaces: Vec<String>,
}

/// Response from `POST /api/v2/assets/canonicalize` (`data` payload).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalizeAssetResponse {
    pub asset: ChainAsset,
    pub input_form: String,
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

// ── Swap execution ────────────────────────────────────────────────────────────

/// Parameters for `POST /api/v1/swap/prepare`.
///
/// Prepare validates the route server-side and returns an unsigned Stellar XDR
/// envelope. Signing happens on the caller's side — the SDK never handles keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapPrepareRequest {
    /// Ordered hops to execute, normally taken from [`Route::path`].
    pub path: Vec<RouteHop>,
    /// Input amount as a decimal string.
    pub amount: String,
    /// G-address of the account submitting the swap.
    pub sender: String,
    /// Minimum acceptable output as a decimal string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_output: Option<String>,
    /// Slippage tolerance in basis points (server default: 50).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u32>,
}

impl SwapPrepareRequest {
    /// Build a prepare request from a ranked [`Route`].
    pub fn from_route(route: &Route, amount: impl Into<String>, sender: impl Into<String>) -> Self {
        Self {
            path: route.path.clone(),
            amount: amount.into(),
            sender: sender.into(),
            min_output: None,
            slippage_bps: None,
        }
    }

    /// Set the slippage tolerance in basis points.
    pub fn slippage_bps(mut self, bps: u32) -> Self {
        self.slippage_bps = Some(bps);
        self
    }

    /// Set the minimum acceptable output amount.
    pub fn min_output(mut self, min_output: impl Into<String>) -> Self {
        self.min_output = Some(min_output.into());
        self
    }
}

/// Response from `POST /api/v1/swap/prepare`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapPrepareResponse {
    /// Base64-encoded unsigned Stellar XDR transaction envelope.
    pub xdr_envelope: String,
    /// Simulated output amount for the prepared route.
    #[serde(default)]
    pub estimated_output: String,
    /// Minimum output enforced by the built transaction.
    #[serde(default)]
    pub min_output: String,
    /// Ledger sequence after which the envelope is no longer valid.
    #[serde(default)]
    pub valid_until_ledger: Option<u64>,
}

/// Parameters for `POST /api/v1/swap/submit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapSubmitRequest {
    /// Base64-encoded **signed** Stellar XDR transaction envelope.
    pub signed_xdr: String,
}

impl SwapSubmitRequest {
    /// Wrap a signed XDR envelope for submission.
    pub fn new(signed_xdr: impl Into<String>) -> Self {
        Self {
            signed_xdr: signed_xdr.into(),
        }
    }
}

/// Response from `POST /api/v1/swap/submit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapSubmitResponse {
    /// Stellar transaction hash.
    pub tx_hash: String,
    /// Submission status: `"pending"`, `"success"`, or `"failed"`.
    pub status: String,
    /// Realized output amount, present once the transaction is finalized.
    #[serde(default)]
    pub output_amount: Option<String>,
    /// Ledger the transaction was included in, when known.
    #[serde(default)]
    pub ledger: Option<u64>,
}

impl SwapSubmitResponse {
    /// Returns `true` when the network has confirmed the swap.
    pub fn is_success(&self) -> bool {
        self.status == "success"
    }
}

// ── Price History ─────────────────────────────────────────────────────────────

/// A single historical price sample returned by the price-history endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceHistoryPoint {
    /// Unix timestamp in milliseconds for the aggregated price bucket.
    pub timestamp: i64,
    /// Average mid-market price for the bucket, encoded as a decimal string.
    pub price: String,
}

/// Response from `GET /api/v1/price-history/{base}/{quote}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceHistoryResponse {
    /// Base asset descriptor.
    pub base_asset: AssetInfo,
    /// Quote asset descriptor.
    pub quote_asset: AssetInfo,
    /// Time window covered by the series, e.g. `"24h"`.
    pub window: String,
    /// Data source description returned by the API.
    pub source: String,
    /// Unix timestamp in milliseconds when the response was generated.
    pub generated_at: i64,
    /// Ordered list of price samples, ascending by timestamp.
    pub points: Vec<PriceHistoryPoint>,
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

// ── Simulate Route ────────────────────────────────────────────────────────────

/// A single hop in the pre-selected route supplied to `simulate_route`.
///
/// Maps to `RouteDryRunHop` in the API's `POST /api/v1/simulate/route` handler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DryRunHop {
    /// Canonical source asset identifier (`"native"` or `"CODE:ISSUER"`).
    pub from_asset: String,
    /// Canonical destination asset identifier.
    pub to_asset: String,
    /// Venue identifier — e.g. `"sdex"` or `"amm:<pool_address>"`.
    pub source: String,
    /// Fee in basis points for this hop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_bps: Option<u32>,
    /// Optional hop price used for dry-run diagnostics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    /// Optional venue reference for per-hop slippage overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub venue_ref: Option<String>,
}

/// Per-venue slippage bound for `simulate_route`.
///
/// When supplied in `SimulateRouteRequest::slippage_bps_overrides`, the given
/// `slippage_bps` applies to the hop whose venue matches `venue_ref` instead
/// of the global `slippage_bps`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlippageOverride {
    /// The `venue_ref` to which this override applies.
    pub venue_ref: String,
    /// Slippage tolerance in basis points for this venue.
    pub slippage_bps: u32,
}

// Wire-shape wrappers — private, used only by SimulateRouteRequest::serialize.
#[derive(Serialize)]
struct RouteWrapper<'a> {
    hops: &'a Vec<DryRunHop>,
}

#[derive(Serialize)]
struct SimulateRouteRequestWire<'a> {
    route: RouteWrapper<'a>,
    amount: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    slippage_bps: Option<u32>,
    slippage_bps_overrides: &'a Vec<SlippageOverride>,
}

/// Request for `POST /api/v1/simulate/route`.
///
/// Serializes to the API wire shape:
/// ```json
/// {
///   "route": { "hops": [...] },
///   "amount": "100.0",
///   "slippage_bps": 50,
///   "slippage_bps_overrides": []
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SimulateRouteRequest {
    /// Ordered list of hops forming the pre-selected route.
    pub hops: Vec<DryRunHop>,
    /// Input amount as a decimal string.
    pub amount: String,
    /// Global slippage tolerance in basis points. Defaults to 50 on the server when `None`.
    pub slippage_bps: Option<u32>,
    /// Per-venue slippage overrides. Defaults to an empty list when absent.
    pub slippage_bps_overrides: Vec<SlippageOverride>,
}

impl Serialize for SimulateRouteRequest {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        SimulateRouteRequestWire {
            route: RouteWrapper { hops: &self.hops },
            amount: &self.amount,
            slippage_bps: self.slippage_bps,
            slippage_bps_overrides: &self.slippage_bps_overrides,
        }
        .serialize(serializer)
    }
}

/// A single hop returned by the routing engine in `SimulateRouteResponse::swap_path`.
///
/// Maps to `SwapHopDto` in the API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwapHopDto {
    /// Canonical source asset identifier.
    pub source_asset: String,
    /// Canonical destination asset identifier.
    pub destination_asset: String,
    /// Venue type: `"sdex"` or `"amm"`.
    pub venue_type: String,
    /// Venue reference string.
    pub venue_ref: String,
    /// Exchange rate at this hop.
    pub price: f64,
    /// Fee in basis points at this hop.
    pub fee_bps: u32,
}

/// The routing-engine swap path returned in `SimulateRouteResponse`.
///
/// Maps to `SwapPathDto` in the API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwapPathDto {
    /// Ordered list of hops in the routing-engine path.
    pub hops: Vec<SwapHopDto>,
    /// Estimated output amount from the routing engine.
    pub estimated_output: i64,
}

/// The quote embedded inside `SimulateRouteResponse`.
///
/// This is a distinct type from `QuoteResponse` to avoid changing the existing
/// type's fields. It mirrors the full API `QuoteResponse` wire shape, using
/// `serde_json::Value` for complex optional sub-types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulateQuoteResult {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    /// Input amount that was quoted.
    pub amount: String,
    /// Effective price (quote asset per base asset unit).
    pub price: String,
    /// Total output amount (`amount × price`).
    pub total: String,
    /// Direction of the quote, e.g. `"sell"`.
    pub quote_type: String,
    /// Whether the quote uses degraded (stale) market data.
    #[serde(default)]
    pub degraded: bool,
    /// Ordered list of hops in the optimal execution path.
    pub path: Vec<PathStep>,
    /// Unix timestamp (ms) when the quote was generated.
    pub timestamp: i64,
    /// Unix timestamp (ms) when this quote expires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// Unix timestamp (ms) of the underlying data source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_timestamp: Option<i64>,
    /// Time-to-live in seconds for client-side staleness detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u32>,
    /// Rationale for quote venue selection (opaque JSON blob).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<serde_json::Value>,
    /// Venues excluded from routing (opaque JSON blob).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusion_diagnostics: Option<serde_json::Value>,
    /// Data freshness metadata (opaque JSON blob).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_freshness: Option<serde_json::Value>,
    /// Market midpoint price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midpoint: Option<String>,
    /// Market spread in basis points.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread_bps: Option<u32>,
    /// Estimated price impact percentage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_impact: Option<String>,
}

/// Response from `POST /api/v1/simulate/route`.
///
/// This is the `data` field extracted from the `ApiResponse<RouteDryRunResponse>`
/// envelope — the envelope metadata (`v`, `timestamp`, `request_id`) is discarded
/// by the SDK client before returning to the caller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulateRouteResponse {
    /// Full quote computed by the simulation pipeline.
    pub quote: SimulateQuoteResult,
    /// Venues excluded from the simulation, if any (opaque JSON blob).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusion_diagnostics: Option<serde_json::Value>,
    /// The routing-engine swap path constructed for this dry-run.
    pub swap_path: SwapPathDto,
}
