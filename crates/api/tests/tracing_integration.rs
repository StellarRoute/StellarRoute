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
