//! OpenAPI documentation

use utoipa::OpenApi;

use crate::models::{
    AssetInfo, BatchItemError, BatchQuoteItemResult, BatchQuoteResponse, CacheMetricsResponse,
    DataFreshness, DependenciesHealthResponse, ErrorResponse, ExcludedVenueInfo,
    ExclusionDiagnostics, ExclusionReason, HealthResponse, OrderbookLevel, OrderbookResponse,
    PairsResponse, PathStep, QuoteRationaleMetadata, QuoteResponse, RouteResponse, TradingPair,
    VenueEvaluation,
};

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::health_check,
        crate::routes::health::dependency_health,
        crate::routes::metrics::cache_metrics,
        crate::routes::pairs::list_pairs,
        crate::routes::pairs::list_markets,
        crate::routes::orderbook::get_orderbook,
        crate::routes::quote::get_quote,
        crate::routes::quote::get_route,
        crate::routes::quote::get_batch_quotes,
        crate::routes::kill_switch::get_kill_switch,
        crate::routes::kill_switch::update_kill_switch,
    ),
    components(schemas(
        HealthResponse,
        DependenciesHealthResponse,
        CacheMetricsResponse,
        PairsResponse,
        TradingPair,
        AssetInfo,
        OrderbookResponse,
        OrderbookLevel,
        QuoteResponse,
        RouteResponse,
        QuoteRationaleMetadata,
        VenueEvaluation,
        PathStep,
        DataFreshness,
        ExclusionDiagnostics,
        ExcludedVenueInfo,
        ExclusionReason,
        BatchQuoteResponse,
        BatchQuoteItemResult,
        BatchItemError,
        ErrorResponse,
        crate::models::request::BatchQuoteRequest,
        crate::models::request::QuoteRequestItem,
        crate::models::request::QuoteType,
        crate::kill_switch::KillSwitchState,
    )),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "trading", description = "Trading and market data endpoints"),
        (name = "admin", description = "Administrative endpoints"),
    ),
    info(
        title = "StellarRoute API",
        version = "0.1.0",
        description = "REST API for DEX aggregation on Stellar Network. \
            Clients may send an optional X-Request-ID header for support correlation; \
            the API echoes the same header in every response.\n\n\
            ## Batch Quotes\n\
            `POST /api/v1/batch/quote` evaluates up to 25 trading pairs concurrently \
            against a shared market snapshot. Per-item failures do not abort the batch.",
        contact(
            name = "StellarRoute",
            url = "https://github.com/stellarroute/stellarroute"
        ),
        license(
            name = "MIT",
        ),
    ),
)]
pub struct ApiDoc;
