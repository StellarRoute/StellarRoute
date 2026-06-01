//! API middleware

pub mod api_versioning;
pub mod deprecation;
pub mod rate_limit;
pub mod request_id;
pub mod tracing;
pub mod validation;

pub use api_versioning::api_versioning_layer;
pub use deprecation::{legacy_route_deprecation, LEGACY_ROUTE_SUNSET, VERSIONING_GUIDE_URL};
pub use rate_limit::{EndpointConfig, RateLimitConfig, RateLimitLayer};
pub use request_id::{request_id_layer, RequestId, REQUEST_ID_HEADER};
pub use tracing::{extract_context_from_headers, inject_context_to_map, trace_layer};
pub use validation::ValidatedQuoteRequest;
