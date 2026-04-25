//! API versioning and deprecation headers.

use axum::{
    body::Body,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

const DEPRECATION_HEADER: &str = "deprecation";
const SUNSET_HEADER: &str = "sunset";
const LINK_HEADER: &str = "link";

fn sunset_value() -> String {
    std::env::var("API_V1_SUNSET").unwrap_or_else(|_| "Wed, 01 Jul 2026 00:00:00 GMT".to_string())
}

fn link_value() -> String {
    std::env::var("API_V1_SUCCESSOR_LINK")
        .unwrap_or_else(|_| "</docs/api/v1-migration-guide>; rel=\"deprecation\"".to_string())
}

fn is_v1_path(path: &str) -> bool {
    path == "/api/v1" || path.starts_with("/api/v1/")
}

pub async fn api_versioning_layer(request: axum::http::Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;

    if is_v1_path(&path) {
        // Don't override headers that were already set by more specific middleware
        // (e.g. legacy-route deprecation that attaches a successor-version Link).
        if !response.headers().contains_key(DEPRECATION_HEADER) {
            if let Ok(value) = HeaderValue::from_str("true") {
                response
                    .headers_mut()
                    .insert(HeaderName::from_static(DEPRECATION_HEADER), value);
            }
        }
        if !response.headers().contains_key(SUNSET_HEADER) {
            if let Ok(value) = HeaderValue::from_str(&sunset_value()) {
                response
                    .headers_mut()
                    .insert(HeaderName::from_static(SUNSET_HEADER), value);
            }
        }
        if !response.headers().contains_key(LINK_HEADER) {
            if let Ok(value) = HeaderValue::from_str(&link_value()) {
                response
                    .headers_mut()
                    .insert(HeaderName::from_static(LINK_HEADER), value);
            }
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, routing::get, Router};
    use tower::ServiceExt;

    #[test]
    fn v1_path_detection_works() {
        assert!(is_v1_path("/api/v1"));
        assert!(is_v1_path("/api/v1/pairs"));
        assert!(!is_v1_path("/health"));
        assert!(!is_v1_path("/api/v2/pairs"));
    }

    #[tokio::test]
    async fn middleware_sets_headers_for_v1_routes() {
        let app = Router::new()
            .route("/api/v1/test", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(api_versioning_layer));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response
                .headers()
                .get("deprecation")
                .and_then(|v| v.to_str().ok()),
            Some("true")
        );
        assert!(response.headers().get("sunset").is_some());
        assert!(response.headers().get("link").is_some());
    }
}
