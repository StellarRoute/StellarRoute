//! Request ID middleware for client/server correlation.

use std::{convert::Infallible, fmt};

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, HeaderValue},
    middleware::Next,
    response::Response,
};

pub const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestId(String);

impl RequestId {
    pub fn from_headers(headers: &HeaderMap) -> Self {
        headers
            .get(REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Self(value.to_string()))
            .unwrap_or_else(Self::generate)
    }

    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn to_header_value(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(&self.0).ok()
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<RequestId>()
            .cloned()
            .unwrap_or_else(|| RequestId::from_headers(&parts.headers)))
    }
}

pub async fn request_id_layer(mut request: axum::http::Request<Body>, next: Next) -> Response {
    let request_id = RequestId::from_headers(request.headers());
    request.extensions_mut().insert(request_id.clone());

    let mut response = next.run(request).await;
    if let Some(value) = request_id.to_header_value() {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_id_uses_incoming_header_when_present() {
        let mut headers = HeaderMap::new();
        headers.insert(
            REQUEST_ID_HEADER,
            HeaderValue::from_static("client-provided-id"),
        );

        let request_id = RequestId::from_headers(&headers);

        assert_eq!(request_id.as_str(), "client-provided-id");
    }

    #[test]
    fn request_id_generates_uuid_when_header_is_missing() {
        let request_id = RequestId::from_headers(&HeaderMap::new());

        assert!(!request_id.as_str().is_empty());
        assert!(uuid::Uuid::parse_str(request_id.as_str()).is_ok());
    }
}
