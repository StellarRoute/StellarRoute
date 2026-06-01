use axum::{
    extract::Request,
    http::{header::LINK, HeaderValue},
    middleware::Next,
    response::Response,
};

pub const LEGACY_ROUTE_SUNSET: &str = "Wed, 01 Jul 2026 00:00:00 GMT";
pub const VERSIONING_GUIDE_URL: &str =
    "https://github.com/StellarRoute/StellarRoute/blob/main/docs/api/versioning.md";

fn successor_path(path_and_query: &str) -> String {
    path_and_query.replacen("/api/v1/route/", "/api/v1/routes/", 1)
}

pub async fn legacy_route_deprecation(request: Request, next: Next) -> Response {
    let successor = request
        .uri()
        .path_and_query()
        .map(|value| successor_path(value.as_str()))
        .unwrap_or_else(|| "/api/v1/routes".to_string());

    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert("deprecation", HeaderValue::from_static("true"));
    headers.insert("sunset", HeaderValue::from_static(LEGACY_ROUTE_SUNSET));

    let link_value = format!(
        "<{}>; rel=\"successor-version\", <{}>; rel=\"deprecation\"",
        successor, VERSIONING_GUIDE_URL
    );
    if let Ok(value) = HeaderValue::from_str(&link_value) {
        headers.insert(LINK, value);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::successor_path;

    #[test]
    fn successor_path_preserves_query_string() {
        let successor = successor_path("/api/v1/route/native/USDC?amount=10&slippage_bps=25");
        assert_eq!(
            successor,
            "/api/v1/routes/native/USDC?amount=10&slippage_bps=25"
        );
    }
}
