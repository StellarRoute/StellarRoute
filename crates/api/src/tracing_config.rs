//! Distributed tracing configuration and initialization.
//!
//! Provides OpenTelemetry-based distributed tracing with context propagation
//! across API, cache, router, and indexer boundaries.
//!
//! # Environment Variables
//!
//! | Variable                  | Values                        | Default          |
//! |--------------------------|-------------------------------|------------------|
//! | `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP collector URL       | (disabled)       |
//! | `OTEL_SERVICE_NAME`      | Service name for spans        | `stellarroute`   |
//! | `OTEL_SAMPLING_RATIO`    | 0.0 to 1.0                    | `1.0`            |
//! | `RUST_LOG`               | tracing filter spec           | `info`           |
//! | `LOG_FORMAT`             | `json` \| `pretty`            | `pretty`         |

use opentelemetry::trace::TraceContextExt;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, Tracer};
use opentelemetry_sdk::Resource;
use std::env;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub service_name: String,
    pub otlp_endpoint: Option<String>,
    pub sampling_ratio: f64,
    pub log_format: LogFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "stellarroute".to_string(),
            otlp_endpoint: None,
            sampling_ratio: 1.0,
            log_format: LogFormat::Pretty,
        }
    }
}

impl TracingConfig {
    pub fn from_env() -> Self {
        let service_name = env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "stellarroute".into());

        let otlp_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();

        let sampling_ratio = env::var("OTEL_SAMPLING_RATIO")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);

        let log_format = match env::var("LOG_FORMAT")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "json" => LogFormat::Json,
            _ => LogFormat::Pretty,
        };

        Self {
            service_name,
            otlp_endpoint,
            sampling_ratio,
            log_format,
        }
    }
}

fn build_tracer(config: &TracingConfig) -> Option<Tracer> {
    let endpoint = config.otlp_endpoint.as_ref()?;

    let sampler = if config.sampling_ratio >= 1.0 {
        Sampler::AlwaysOn
    } else if config.sampling_ratio <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sampling_ratio)
    };

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint);

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    config.service_name.clone(),
                )])),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .ok()?;

    Some(tracer)
}

pub fn init() {
    init_with_config(TracingConfig::from_env());
}

pub fn init_with_config(config: TracingConfig) {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = match config.log_format {
        LogFormat::Json => fmt::layer().json().boxed(),
        LogFormat::Pretty => fmt::layer().boxed(),
    };

    let registry = tracing_subscriber::registry().with(filter).with(fmt_layer);

    if let Some(tracer) = build_tracer(&config) {
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        registry.with(otel_layer).init();
        tracing::info!(
            service_name = %config.service_name,
            sampling_ratio = config.sampling_ratio,
            "Distributed tracing enabled"
        );
    } else {
        registry.init();
        tracing::info!("Tracing initialized (OTLP export disabled)");
    }
}

pub fn shutdown() {
    global::shutdown_tracer_provider();
}

#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
}

impl TraceContext {
    pub fn current() -> Self {
        let span = Span::current();
        let ctx = span.context();
        let span_ref = ctx.span();
        let span_ctx = span_ref.span_context();

        Self {
            trace_id: span_ctx.trace_id().to_string(),
            span_id: span_ctx.span_id().to_string(),
        }
    }

    pub fn from_headers(headers: &axum::http::HeaderMap) -> Option<Self> {
        let traceparent = headers.get("traceparent")?.to_str().ok()?;
        let parts: Vec<&str> = traceparent.split('-').collect();
        if parts.len() >= 3 {
            Some(Self {
                trace_id: parts[1].to_string(),
                span_id: parts[2].to_string(),
            })
        } else {
            None
        }
    }

    pub fn inject_headers(&self, headers: &mut axum::http::HeaderMap) {
        let traceparent = format!("00-{}-{}-01", self.trace_id, self.span_id);
        if let Ok(val) = traceparent.parse() {
            headers.insert("traceparent", val);
        }
    }
}

pub mod span_names {
    pub const QUOTE_REQUEST: &str = "quote.request";
    pub const QUOTE_COMPUTE: &str = "quote.compute";
    pub const CACHE_LOOKUP: &str = "cache.lookup";
    pub const CACHE_STORE: &str = "cache.store";
    pub const ROUTE_SEARCH: &str = "route.search";
    pub const ROUTE_OPTIMIZE: &str = "route.optimize";
    pub const DB_QUERY: &str = "db.query";
    pub const INDEXER_FETCH: &str = "indexer.fetch";
    pub const INDEXER_PROCESS: &str = "indexer.process";
    pub const GRAPH_FILTER: &str = "graph.filter";
    pub const HEALTH_CHECK: &str = "health.check";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "stellarroute");
        assert!(config.otlp_endpoint.is_none());
        assert!((config.sampling_ratio - 1.0).abs() < f64::EPSILON);
        assert_eq!(config.log_format, LogFormat::Pretty);
    }

    #[test]
    fn test_sampling_ratio_clamped() {
        let config = TracingConfig {
            sampling_ratio: 1.5,
            ..Default::default()
        };
        let clamped = config.sampling_ratio.clamp(0.0, 1.0);
        assert!((clamped - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_trace_context_inject_headers() {
        let ctx = TraceContext {
            trace_id: "00112233445566778899aabbccddeeff".to_string(),
            span_id: "0011223344556677".to_string(),
        };

        let mut headers = axum::http::HeaderMap::new();
        ctx.inject_headers(&mut headers);

        let traceparent = headers.get("traceparent").unwrap().to_str().unwrap();
        assert!(traceparent.contains(&ctx.trace_id));
        assert!(traceparent.contains(&ctx.span_id));
    }

    #[test]
    fn test_trace_context_from_headers() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "traceparent",
            "00-00112233445566778899aabbccddeeff-0011223344556677-01"
                .parse()
                .unwrap(),
        );

        let ctx = TraceContext::from_headers(&headers).unwrap();
        assert_eq!(ctx.trace_id, "00112233445566778899aabbccddeeff");
        assert_eq!(ctx.span_id, "0011223344556677");
    }
}
