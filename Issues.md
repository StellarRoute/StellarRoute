Summary
Expand docs/api/error_taxonomy.md into a practical integrator guide covering retry semantics, SDK error mapping for both JS and Rust SDKs, and user-facing error presentation patterns.

Motivation
The error taxonomy catalog exists but stops at a brief JS SDK mapping table. Integrators need guidance on which errors are retryable, how to handle stale_market_data and overloaded, and how to surface errors in UI/CLI contexts.

Acceptance Criteria
 Extend docs/api/error_taxonomy.md (or add docs/api/integrator-error-guide.md) with:
Retry vs fail-fast matrix per error code
Recommended backoff for rate_limit_exceeded and overloaded
Handling stale_market_data (refresh quote flow)
JS SDK examples using StellarRouteApiError helpers
Rust SDK error type mapping examples
Sample JSON error responses for each code
 Document deprecation/version headers interaction with errors (link to docs/api/versioning-policy.md)
 Link from docs/sdk-js/README.md and Rust SDK docs
 Frontend contributors: note mapping to trader-facing copy (link to design error copy issue if open)
Out of Scope
Changing API error codes or HTTP status mappings
i18n implementation
References
docs/api/error_taxonomy.md
docs/api/versioning-policy.md
sdk-js/src/ error handling