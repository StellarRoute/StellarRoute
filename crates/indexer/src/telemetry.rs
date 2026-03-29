//! Structured logging and distributed tracing for the indexer.
//!
//! # Environment variables
//!
//! | Variable                      | Values              | Default        |
//! |------------------------------|---------------------|----------------|
//! | `RUST_LOG`                   | tracing filter spec | `info`         |
//! | `LOG_FORMAT`                 | `json` \| `pretty`  | `pretty`       |
//! | `OTEL_EXPORTER_OTLP_ENDPOINT`| OTLP collector URL  | (disabled)     |
//! | `OTEL_SERVICE_NAME`          | Service name        | `stellarroute-indexer` |
//! | `OTEL_SAMPLING_RATIO`        | 0.0 to 1.0          | `1.0`          |
//!
//! ## Examples
//!
//! ```bash
//! # Development
//! RUST_LOG=stellarroute_indexer=debug ./stellarroute-indexer
//!
//! # Production with OTLP export
//! RUST_LOG=info LOG_FORMAT=json OTEL_EXPORTER_OTLP_ENDPOINT=http://collector:4317 ./stellarroute-indexer
//! ```

use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, Tracer};
use opentelemetry_sdk::Resource;
use std::env;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub service_name: String,
    pub otlp_endpoint: Option<String>,
    pub sampling_ratio: f64,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "stellarroute-indexer".to_string(),
            otlp_endpoint: None,
            sampling_ratio: 1.0,
        }
    }
}

impl TracingConfig {
    pub fn from_env() -> Self {
        let service_name =
            env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "stellarroute-indexer".into());

        let otlp_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();

        let sampling_ratio = env::var("OTEL_SAMPLING_RATIO")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);

        Self {
            service_name,
            otlp_endpoint,
            sampling_ratio,
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

    let log_format = env::var("LOG_FORMAT").unwrap_or_default();
    let fmt_layer = if log_format.to_lowercase() == "json" {
        fmt::layer().json().boxed()
    } else {
        fmt::layer().boxed()
    };

    let registry = tracing_subscriber::registry().with(filter).with(fmt_layer);

    if let Some(tracer) = build_tracer(&config) {
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        registry.with(otel_layer).init();
        tracing::info!(
            service_name = %config.service_name,
            sampling_ratio = config.sampling_ratio,
            "Distributed tracing enabled for indexer"
        );
    } else {
        registry.init();
        tracing::info!("Indexer tracing initialized (OTLP export disabled)");
    }
}

pub fn shutdown() {
    global::shutdown_tracer_provider();
}
