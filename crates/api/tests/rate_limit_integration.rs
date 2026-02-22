//! Integration tests for the rate limiting middleware.
//!
//! All tests here run **without external dependencies** (no Postgres, no
//! Redis). The rate limiter is exercised through its in-memory backend and
//! through the full Axum router using `tower::ServiceExt::oneshot`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use stellarroute_api::middleware::{EndpointConfig, RateLimitConfig, RateLimitLayer};
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Helper: build a minimal Axum router with an in-memory rate limiter
// ---------------------------------------------------------------------------

fn build_test_router(endpoint_config: EndpointConfig) -> axum::Router {
    use axum::{routing::get, Router};

    // Simple health-style handler that always returns 200
    async fn ok_handler() -> &'static str {
        "ok"
    }

    Router::new()
        .route("/health", get(ok_handler))
        .route("/api/v1/pairs", get(ok_handler))
        .route("/api/v1/orderbook/:b/:q", get(ok_handler))
        .route("/api/v1/quote/:b/:q", get(ok_handler))
        .layer(RateLimitLayer::in_memory(endpoint_config))
}

// ---------------------------------------------------------------------------
// Configuration unit tests
// ---------------------------------------------------------------------------

#[test]
fn rate_limit_config_default_values() {
    let cfg = RateLimitConfig::default();
    assert_eq!(cfg.max_requests, 200);
    assert_eq!(cfg.window.as_secs(), 60);
}

#[test]
fn endpoint_config_selects_pairs_limit() {
    std::env::remove_var("RATE_LIMIT_PAIRS");
    std::env::remove_var("RATE_LIMIT_ORDERBOOK");
    std::env::remove_var("RATE_LIMIT_QUOTE");
    std::env::remove_var("RATE_LIMIT_WINDOW_SECS");

    let cfg = EndpointConfig::default();
    assert_eq!(cfg.for_path("/api/v1/pairs").max_requests, 60);
}

#[test]
fn endpoint_config_selects_orderbook_limit() {
    std::env::remove_var("RATE_LIMIT_PAIRS");
    std::env::remove_var("RATE_LIMIT_ORDERBOOK");
    std::env::remove_var("RATE_LIMIT_QUOTE");
    std::env::remove_var("RATE_LIMIT_WINDOW_SECS");

    let cfg = EndpointConfig::default();
    assert_eq!(cfg.for_path("/api/v1/orderbook/XLM/USDC").max_requests, 30);
}

#[test]
fn endpoint_config_selects_quote_limit() {
    std::env::remove_var("RATE_LIMIT_PAIRS");
    std::env::remove_var("RATE_LIMIT_ORDERBOOK");
    std::env::remove_var("RATE_LIMIT_QUOTE");
    std::env::remove_var("RATE_LIMIT_WINDOW_SECS");

    let cfg = EndpointConfig::default();
    assert_eq!(cfg.for_path("/api/v1/quote/XLM/USDC").max_requests, 100);
}

#[test]
fn endpoint_config_selects_default_for_health() {
    std::env::remove_var("RATE_LIMIT_PAIRS");
    std::env::remove_var("RATE_LIMIT_ORDERBOOK");
    std::env::remove_var("RATE_LIMIT_QUOTE");
    std::env::remove_var("RATE_LIMIT_WINDOW_SECS");

    let cfg = EndpointConfig::default();
    assert_eq!(cfg.for_path("/health").max_requests, 200);
}

// ---------------------------------------------------------------------------
// HTTP-level: headers present on allowed requests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rate_limit_headers_present_on_allowed_request() {
    let cfg = EndpointConfig::default();
    let router = build_test_router(cfg);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let headers = response.headers();
    assert!(
        headers.contains_key("x-ratelimit-limit"),
        "missing X-RateLimit-Limit"
    );
    assert!(
        headers.contains_key("x-ratelimit-remaining"),
        "missing X-RateLimit-Remaining"
    );
    assert!(
        headers.contains_key("x-ratelimit-reset"),
        "missing X-RateLimit-Reset"
    );
}

#[tokio::test]
async fn rate_limit_remaining_is_numeric() {
    let cfg = EndpointConfig::default();
    let router = build_test_router(cfg);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/pairs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let remaining = response
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());

    assert!(
        remaining.is_some(),
        "X-RateLimit-Remaining must be a number"
    );
}

#[tokio::test]
async fn rate_limit_limit_header_matches_endpoint_config() {
    // pairs endpoint → limit 60
    std::env::remove_var("RATE_LIMIT_PAIRS");
    std::env::remove_var("RATE_LIMIT_WINDOW_SECS");

    let cfg = EndpointConfig::default();
    let router = build_test_router(cfg);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/pairs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let limit: u64 = response
        .headers()
        .get("x-ratelimit-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .expect("X-RateLimit-Limit must be numeric");

    assert_eq!(limit, 60, "pairs limit should be 60");
}

// ---------------------------------------------------------------------------
// HTTP-level: 429 when limit exceeded
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rate_limit_returns_429_after_limit_exceeded() {
    use std::time::Duration;

    // Low limit so the test is fast
    let cfg = EndpointConfig {
        pairs: RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(60),
        },
        orderbook: RateLimitConfig {
            max_requests: 30,
            window: Duration::from_secs(60),
        },
        quote: RateLimitConfig {
            max_requests: 100,
            window: Duration::from_secs(60),
        },
        default: RateLimitConfig {
            max_requests: 200,
            window: Duration::from_secs(60),
        },
    };

    let layer = RateLimitLayer::in_memory(cfg);

    use axum::{routing::get, Router};
    async fn ok() -> &'static str {
        "ok"
    }

    let router = Router::new().route("/api/v1/pairs", get(ok)).layer(layer);

    // First two requests should succeed
    for _ in 0..2 {
        let resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/pairs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Third request must be denied
    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/pairs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    // Response headers
    let headers = resp.headers().clone();
    assert!(headers.contains_key("x-ratelimit-limit"));
    assert!(headers.contains_key("x-ratelimit-remaining"));
    assert!(headers.contains_key("retry-after"));

    let remaining: u64 = headers
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .expect("X-RateLimit-Remaining must be numeric");
    assert_eq!(remaining, 0);

    // Body must be JSON with the error key
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).expect("body must be JSON");
    assert_eq!(
        json["error"], "rate_limit_exceeded",
        "error key must be rate_limit_exceeded"
    );
    assert!(
        json["message"].as_str().is_some(),
        "message must be present"
    );
}

#[tokio::test]
async fn rate_limit_429_content_type_is_json() {
    use std::time::Duration;

    let cfg = EndpointConfig {
        pairs: RateLimitConfig {
            max_requests: 1,
            window: Duration::from_secs(60),
        },
        orderbook: RateLimitConfig {
            max_requests: 30,
            window: Duration::from_secs(60),
        },
        quote: RateLimitConfig {
            max_requests: 100,
            window: Duration::from_secs(60),
        },
        default: RateLimitConfig {
            max_requests: 200,
            window: Duration::from_secs(60),
        },
    };

    let layer = RateLimitLayer::in_memory(cfg);

    use axum::{routing::get, Router};
    async fn ok() -> &'static str {
        "ok"
    }
    let router = Router::new().route("/api/v1/pairs", get(ok)).layer(layer);

    // Exhaust limit (1 request)
    router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/pairs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Second is denied
    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/pairs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.contains("application/json"), "got content-type: {ct}");
}

// ---------------------------------------------------------------------------
// x-forwarded-for header respected
// ---------------------------------------------------------------------------

#[tokio::test]
async fn different_ips_have_independent_limits() {
    use std::time::Duration;

    // Set a very low limit so we can exhaust it quickly with one IP
    let cfg = EndpointConfig {
        pairs: RateLimitConfig {
            max_requests: 1,
            window: Duration::from_secs(60),
        },
        orderbook: RateLimitConfig {
            max_requests: 30,
            window: Duration::from_secs(60),
        },
        quote: RateLimitConfig {
            max_requests: 100,
            window: Duration::from_secs(60),
        },
        default: RateLimitConfig {
            max_requests: 200,
            window: Duration::from_secs(60),
        },
    };

    use axum::{routing::get, Router};
    async fn ok() -> &'static str {
        "ok"
    }
    let router = Router::new()
        .route("/api/v1/pairs", get(ok))
        .layer(RateLimitLayer::in_memory(cfg));

    // IP A: exhaust limit
    router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/pairs")
                .header("x-forwarded-for", "10.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // IP A: second request → 429
    let denied = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/pairs")
                .header("x-forwarded-for", "10.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::TOO_MANY_REQUESTS);

    // IP B: first request → 200 (different IP, independent counter)
    let allowed = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/pairs")
                .header("x-forwarded-for", "10.0.0.2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);
}
