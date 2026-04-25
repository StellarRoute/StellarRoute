//! Distributed tracing middleware for request context propagation.

use axum::{body::Body, extract::Request, http::HeaderMap, middleware::Next, response::Response};
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use std::collections::HashMap;
use tracing::{info_span, Instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> opentelemetry::propagation::Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

struct HeaderInjector<'a>(&'a mut HashMap<String, String>);

impl<'a> opentelemetry::propagation::Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

pub fn extract_context_from_headers(headers: &HeaderMap) -> opentelemetry::Context {
    let propagator = TraceContextPropagator::new();
    propagator.extract(&HeaderExtractor(headers))
}

pub fn inject_context_to_map(ctx: &opentelemetry::Context, map: &mut HashMap<String, String>) {
    let propagator = TraceContextPropagator::new();
    propagator.inject_context(ctx, &mut HeaderInjector(map));
}

pub async fn trace_layer(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().path().to_string();
    let parent_ctx = extract_context_from_headers(request.headers());

    let span = info_span!(
        "http.request",
        http.method = %method,
        http.target = %uri,
        http.status_code = tracing::field::Empty,
        otel.kind = "server",
    );

    span.set_parent(parent_ctx);

    async move {
        let response = next.run(request).await;
        Span::current().record("http.status_code", response.status().as_u16());
        response
    }
    .instrument(span)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use opentelemetry::trace::TraceContextExt;

    #[test]
    fn test_extract_context_no_headers() {
        let headers = HeaderMap::new();
        let ctx = extract_context_from_headers(&headers);
        assert!(
            ctx.span().span_context().trace_id().to_string() == "00000000000000000000000000000000"
        );
    }

    #[test]
    fn test_extract_context_with_traceparent() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "traceparent",
            HeaderValue::from_static("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        );
        let ctx = extract_context_from_headers(&headers);
        let span = ctx.span();
        let span_ctx = span.span_context();
        assert_eq!(
            span_ctx.trace_id().to_string(),
            "0af7651916cd43dd8448eb211c80319c"
        );
    }

    #[test]
    fn test_inject_context_to_map() {
        let mut map = HashMap::new();
        let ctx = opentelemetry::Context::current();
        inject_context_to_map(&ctx, &mut map);
        assert!(map.is_empty() || map.contains_key("traceparent"));
    }
}
