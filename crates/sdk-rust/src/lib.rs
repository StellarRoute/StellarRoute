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
    AgentHealth, ApiV2Info, AssetInfo, AssetMetadataResponse, BatchQuoteRequest,
    BatchQuoteResponse, BridgeVenueMeta, CanonicalizeAssetResponse, CardApplicationDraft,
    CardApplicationValidation, CardAuthorization, CardAuthorizationsResponse, CardHealth,
    CctpDirection, CctpFeeQuote, CctpFinality, CctpPrepareBurnResponse, CctpPrepareMintResponse,
    CctpQuoteRequest, CctpQuoteResponse, CctpReattestResponse, CctpStatusDetails,
    CctpSubmitBurnRequest, CctpSubmitBurnResponse, CctpSubmitMintRequest, CctpSubmitMintResponse,
    CctpTransferStatus, CctpTransferStatusResponse, ChainAsset, HealthResponse, OrderbookLevel,
    OrderbookResponse, PairsResponse, PathStep, PreparedWalletPayload, QuoteRequest,
    QuoteRequestItem, QuoteResponse, QuoteType, Route, RouteHop, RoutesRequest, RoutesResponse,
    SupportedCorridor, SwapPrepareRequest, SwapPrepareResponse, SwapSubmitRequest,
    SwapSubmitResponse, TradingPair, ValidateIntentRequest, ValidateIntentResponse,
    CCTP_IDEMPOTENCY_HEADER, CCTP_PROVIDER_ID, CCTP_TESTNET_CORRIDOR_ID,
    CCTP_TRANSFER_ACCESS_HEADER,
};
