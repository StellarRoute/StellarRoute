//! OpenAPI documentation

use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::Modify;
use utoipa::OpenApi;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "admin_token",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-admin-token"))),
            );
            components.add_security_scheme(
                "admin_bearer",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}

use crate::models::{
    AssetInfo, AssetMetadataBulkResponse, AssetMetadataResponse, BatchItemError,
    BatchQuoteItemResult, BatchQuoteResponse, CacheFlushResponse, CacheMetricsResponse,
    DataFreshness, DataQuality, DependenciesHealthResponse, ErrorResponse, ExcludedVenueInfo,
    ExclusionDiagnostics, ExclusionReason, HealthResponse, OrderbookLevel, OrderbookResponse,
    PairsResponse, PathStep, PriceHistoryPoint, PriceHistoryResponse,
    QuoteExpirationWebhookPayload, QuoteExpirationWebhookRegistrationResponse,
    QuoteRationaleMetadata, QuoteResponse, RouteResponse, TradingPair, VenueEvaluation,
};

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::health_check,
        crate::routes::health::dependency_health,
        crate::routes::metrics::cache_metrics,
        crate::routes::assets::get_asset_metadata,
        crate::routes::assets::list_assets_metadata,
        crate::routes::activity::list_swap_activity,
        crate::routes::pairs::list_pairs,
        crate::routes::pairs::list_markets,
        crate::routes::orderbook::get_orderbook,
        crate::routes::orderbook::get_batch_orderbooks,
        crate::routes::price_history::get_price_history,
        crate::routes::quote::get_quote,
        crate::routes::quote::get_route,
        crate::routes::route_graph_version::get_route_graph_version,
        crate::routes::admin::flush_cache,
        crate::routes::admin_cache::flush_cache,
        crate::routes::quote::get_batch_quotes,
        crate::routes::integrator_webhooks::upsert_quote_expiration_webhook,
        crate::routes::kill_switch::get_kill_switch,
        crate::routes::kill_switch::update_kill_switch,
        crate::routes::swap::prepare_swap,
        crate::routes::swap::submit_swap,
        crate::routes::card::card_health,
        crate::routes::card::validate_card_application,
        crate::routes::card::list_card_authorizations,
        crate::routes::v2::api_v2_info,
        crate::routes::v2::canonicalize_asset,
        crate::routes::v2_cctp::cctp_quote,
        crate::routes::v2_cctp::cctp_prepare_burn,
        crate::routes::v2_cctp::cctp_submit_burn,
        crate::routes::v2_cctp::cctp_get_transfer,
        crate::routes::v2_cctp::cctp_prepare_mint,
        crate::routes::v2_cctp::cctp_submit_mint,
        crate::routes::v2_cctp::cctp_reattest,
        crate::routes::agent::agent_health,
        crate::routes::agent::agent_tools,
        crate::routes::agent::agent_intents_validate,
    ),
    components(schemas(
        crate::routes::admin_cache::CacheFlushRequest,
        HealthResponse,
        DependenciesHealthResponse,
        CacheMetricsResponse,
        CacheFlushResponse,
        AssetMetadataResponse,
        AssetMetadataBulkResponse,
        crate::routes::activity::SwapActivityItem,
        crate::routes::activity::SwapActivityResponse,
        PairsResponse,
        TradingPair,
        AssetInfo,
        OrderbookResponse,
        OrderbookLevel,
        crate::models::OrderbookSummary,
        QuoteResponse,
        PriceHistoryPoint,
        PriceHistoryResponse,
        RouteResponse,
        crate::routes::route_graph_version::RouteGraphVersionResponse,
        QuoteRationaleMetadata,
        VenueEvaluation,
        PathStep,
        DataFreshness,
        DataQuality,
        ExclusionDiagnostics,
        ExcludedVenueInfo,
        ExclusionReason,
        BatchQuoteResponse,
        BatchQuoteItemResult,
        BatchItemError,
        QuoteExpirationWebhookRegistrationResponse,
        QuoteExpirationWebhookPayload,
        ErrorResponse,
        crate::models::request::BatchQuoteRequest,
        crate::models::request::QuoteRequestItem,
        crate::models::request::BatchOrderbookRequest,
        crate::models::request::OrderbookRequestItem,
        crate::models::response::BatchOrderbookResponse,
        crate::models::response::BatchOrderbookItemResult,
        crate::models::request::QuoteType,
        crate::models::request::QuoteExpirationWebhookRegistrationRequest,
        crate::models::request::AssetPath,
        crate::models::request::SwapPrepareRequest,
        crate::models::request::SwapSubmitRequest,
        crate::models::response::SwapPrepareResponse,
        crate::models::response::SwapSubmitResponse,
        crate::routes::card::CardHealth,
        crate::routes::card::CardApplicationDraft,
        crate::agent::AgentTool,
        crate::agent::AgentToolsResponse,
        crate::routes::card::CardApplicationValidation,
        crate::routes::card::CardAuthorization,
        crate::routes::card::CardAuthorizationsResponse,
        crate::routes::simulation_route::RouteDryRunPath,
        crate::routes::simulation_route::RouteDryRunHop,
        crate::models::v2::ApiV2Info,
        crate::models::v2::ChainAssetV2,
        crate::models::v2::CanonicalizeAssetRequest,
        crate::models::v2::CanonicalizeAssetResponse,
        crate::models::v2_cctp::CctpChainAsset,
        crate::models::v2_cctp::SupportedCorridor,
        crate::models::v2_cctp::CctpDirection,
        crate::models::v2_cctp::CctpFinality,
        crate::models::v2_cctp::CctpTransferStatus,
        crate::models::v2_cctp::CctpFeeQuote,
        crate::models::v2_cctp::PreparedWalletPayload,
        crate::models::v2_cctp::CctpStatusDetails,
        crate::models::v2_cctp::CctpQuoteRequest,
        crate::models::v2_cctp::CctpQuoteResponse,
        crate::models::v2_cctp::CctpTransferStatusResponse,
        crate::models::v2_cctp::CctpPrepareBurnResponse,
        crate::models::v2_cctp::CctpSubmitBurnRequest,
        crate::models::v2_cctp::CctpSubmitBurnResponse,
        crate::models::v2_cctp::CctpPrepareMintResponse,
        crate::models::v2_cctp::CctpSubmitMintRequest,
        crate::models::v2_cctp::CctpSubmitMintResponse,
        crate::models::v2_cctp::CctpReattestResponse,
        crate::routes::agent::AgentHealthResponse,
    )),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "assets", description = "Asset metadata endpoints"),
        (name = "activity", description = "Indexed contract activity endpoints"),
        (name = "trading", description = "Trading and market data endpoints"),
        (name = "admin", description = "Administrative endpoints"),
        (name = "integrator", description = "Integrator configuration and webhook endpoints"),
        (name = "swap", description = "Live swap path: prepare an unsigned transaction from a route, then submit a signed transaction on-chain"),
        (name = "card", description = "Card program preview (flag-gated behind CARD_ENABLED; 404 when disabled; never holds keys or card PANs)"),
        (name = "v2", description = "API v2 seam: chain-aware asset canonicalize/info only (no v2 quotes). Bridges are metadata-only/non-executable; provider controls apply only to provider-tagged candidates."),
        (name = "cctp", description = "Circle CCTP v2 bridge contract (fail-closed until backend execution is enabled). Burn/mint prepare returns wallet payloads; submit endpoints accept tx hash acknowledgement only."),
        (name = "agent", description = "AI agent feature endpoints (fail-closed; return 404 when AI_AGENT_ENABLED is unset). No trade amounts or wallet addresses are stored."),
    ),
    info(
        title = "StellarRoute API",
        version = "0.1.0",
        description = "REST API for DEX aggregation on Stellar Network. \
            Clients may send an optional X-Request-ID header for support correlation; \
            the API echoes the same header in every response. Large quote responses \
            honor `Accept-Encoding: br` or `Accept-Encoding: gzip`; smaller quote \
            responses are left uncompressed, and Prometheus metrics track original \
            versus wire response bytes.\n\n\
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
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::ApiDoc;
    use utoipa::OpenApi;

    /// LIVE-02: the quote data-quality vocabulary must be documented in the
    /// generated OpenAPI spec, as a closed set of string enum values.
    #[test]
    fn openapi_documents_data_quality_enum() {
        let spec = serde_json::to_value(ApiDoc::openapi()).expect("serialize openapi");
        let schema = &spec["components"]["schemas"]["DataQuality"];

        assert!(
            schema.is_object(),
            "DataQuality schema must be present in components.schemas, got {schema}"
        );

        let variants = schema["enum"]
            .as_array()
            .expect("DataQuality schema must declare an enum array");
        let names: Vec<&str> = variants.iter().filter_map(|v| v.as_str()).collect();

        for expected in ["live", "degraded", "stale", "unknown"] {
            assert!(
                names.contains(&expected),
                "DataQuality enum must document `{expected}`, got {names:?}"
            );
        }
        assert_eq!(names.len(), 4, "DataQuality enum must be closed: {names:?}");
    }

    /// The quote endpoint's documented 200 response must reference the
    /// data-quality vocabulary so clients can discover it from the operation.
    #[test]
    fn quote_endpoint_references_data_quality_vocabulary() {
        let spec = serde_json::to_value(ApiDoc::openapi()).expect("serialize openapi");
        let description = spec["paths"]["/api/v1/quote/{base}/{quote}"]["get"]["responses"]["200"]
            ["description"]
            .as_str()
            .unwrap_or_default();

        assert!(
            description.contains("DataQuality"),
            "quote 200 response must document the data-quality vocabulary, got {description:?}"
        );
    }
}
