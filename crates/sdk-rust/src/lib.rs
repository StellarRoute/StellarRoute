//! # StellarRoute Rust SDK
//!
//! Async Rust client for the StellarRoute DEX aggregation API.
//!
//! ## Quick start
//!
//! ```no_run
//! use stellarroute_sdk::{ClientBuilder, QuoteRequest};
//!
//! #[tokio::main]
//! async fn main() -> stellarroute_sdk::Result<()> {
//!     let client = ClientBuilder::new("http://localhost:3000").build()?;
//!
//!     // Health check
//!     let health = client.health().await?;
//!     assert!(health.is_healthy());
//!
//!     // Price quote
//!     let quote = client.quote(QuoteRequest::sell("native", "USDC")).await?;
//!     println!("{} XLM = {} USDC", quote.amount, quote.total);
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod error;
pub mod types;

// Flat re-exports — callers only need `use stellarroute_sdk::*`.
pub use client::{Client, ClientBuilder, StellarRouteClient};
pub use error::{ApiErrorCode, RateLimitInfo, Result, SdkError};
pub use types::{
    ApiV2Info, AssetInfo, BridgeVenueMeta, CanonicalizeAssetResponse, ChainAsset, DryRunHop,
    HealthResponse, OrderbookLevel, OrderbookResponse, PairsResponse, PathStep, PriceHistoryPoint,
    PriceHistoryResponse, QuoteRequest, QuoteResponse, QuoteType, Route, RouteHop, RoutesRequest,
    RoutesResponse, SimulateQuoteResult, SimulateRouteRequest, SimulateRouteResponse,
    SlippageOverride, SwapHopDto, SwapPathDto, SwapPrepareRequest, SwapPrepareResponse,
    SwapSubmitRequest, SwapSubmitResponse, TradingPair,
};
