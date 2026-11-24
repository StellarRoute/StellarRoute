//! Integration tests for distributed tracing.

use stellarroute_api::tracing_config::{LogFormat, TracingConfig};

#[test]
fn test_tracing_config_defaults() {
    let config = TracingConfig::default();
    assert_eq!(config.service_name, "stellarroute");
    assert!(config.otlp_endpoint.is_none());
    assert!((config.sampling_ratio - 1.0).abs() < f64::EPSILON);
    assert_eq!(config.log_format, LogFormat::Pretty);
}

#[test]
fn test_tracing_config_from_env() {
    std::env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    std::env::remove_var("OTEL_SERVICE_NAME");
    std::env::remove_var("OTEL_SAMPLING_RATIO");

    let config = TracingConfig::from_env();
    assert_eq!(config.service_name, "stellarroute");
    assert!(config.otlp_endpoint.is_none());
}

#[test]
fn test_sampling_ratio_bounds() {
    let config = TracingConfig {
        service_name: "test".to_string(),
        otlp_endpoint: None,
        sampling_ratio: 1.5,
        log_format: LogFormat::Pretty,
    };
    let clamped = config.sampling_ratio.clamp(0.0, 1.0);
    assert!((clamped - 1.0).abs() < f64::EPSILON);

    let config_low = TracingConfig {
        service_name: "test".to_string(),
        otlp_endpoint: None,
        sampling_ratio: -0.5,
        log_format: LogFormat::Pretty,
    };
    let clamped_low = config_low.sampling_ratio.clamp(0.0, 1.0);
    assert!(clamped_low.abs() < f64::EPSILON);
}

#[test]
fn test_log_format_variants() {
    assert_eq!(LogFormat::Json, LogFormat::Json);
    assert_eq!(LogFormat::Pretty, LogFormat::Pretty);
    assert_ne!(LogFormat::Json, LogFormat::Pretty);
}

#[derive(Default, Clone)]
struct TestSpanCollector {
    spans: std::sync::Arc<std::sync::Mutex<Vec<CapturedSpan>>>,
}

#[derive(Debug, Clone)]
struct CapturedSpan {
    name: String,
    fields: std::collections::HashMap<String, String>,
}

struct Visitor<'a>(&'a mut std::collections::HashMap<String, String>);

impl<'a> tracing::field::Visit for Visitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_string(), format!("{:?}", value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for TestSpanCollector {
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        _id: &tracing::span::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut fields = std::collections::HashMap::new();
        attrs.record(&mut Visitor(&mut fields));
        self.spans.lock().unwrap().push(CapturedSpan {
            name: attrs.metadata().name().to_string(),
            fields,
        });
    }

    fn on_record(
        &self,
        _id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor_fields = std::collections::HashMap::new();
        values.record(&mut Visitor(&mut visitor_fields));
        let mut spans = self.spans.lock().unwrap();
        if let Some(span) = spans.iter_mut().last() {
            for (k, v) in visitor_fields {
                span.fields.insert(k, v);
            }
        }
    }
}

#[test]
fn test_quote_span_attributes_and_names() {
    use tracing_subscriber::layer::SubscriberExt;

    let collector = TestSpanCollector::default();
    let subscriber = tracing_subscriber::registry().with(collector.clone());

    tracing::subscriber::with_default(subscriber, || {
        let base = "native";
        let quote = "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let request_id = "req-test-123";
        let pair = format!("{base}:{quote}");

        let span = tracing::info_span!(
            "quote_pipeline",
            request_id = %request_id,
            %base,
            %quote,
            %pair,
            cache_hit = false,
            error_class = tracing::field::Empty,
            latency_ms = tracing::field::Empty,
        );

        let _guard = span.enter();
        tracing::Span::current().record("cache_hit", true);
        tracing::Span::current().record("error_class", "none");
        tracing::Span::current().record("latency_ms", 12u64);
    });

    let captured = collector.spans.lock().unwrap();
    let quote_span = captured
        .iter()
        .find(|s| s.name == "quote_pipeline")
        .expect("quote_pipeline span must exist");

    assert_eq!(quote_span.name, "quote_pipeline");
    assert!(quote_span.fields.contains_key("pair"));
    assert_eq!(
        quote_span.fields.get("pair").unwrap(),
        "native:USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
    );
    assert_eq!(quote_span.fields.get("cache_hit").unwrap(), "true");
}

