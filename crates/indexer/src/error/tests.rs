use super::*;

#[test]
fn test_error_display() {
    let err = IndexerError::InvalidAsset {
        asset: "INVALID".to_string(),
        reason: "Unknown type".to_string(),
    };
    assert!(err.to_string().contains("INVALID"));
    assert!(err.to_string().contains("Unknown type"));
}

#[test]
fn test_database_connection_error() {
    let err = IndexerError::DatabaseConnection("Failed to connect".to_string());
    assert_eq!(err.log_level(), tracing::Level::ERROR);
    assert!(!err.is_retryable());
}

#[test]
fn test_rate_limit_error() {
    let err = IndexerError::RateLimitExceeded {
        retry_after: Some(60),
    };
    assert_eq!(err.log_level(), tracing::Level::WARN);
    assert!(err.is_retryable());
}

#[test]
fn test_network_timeout_retryable() {
    let err = IndexerError::NetworkTimeout {
        timeout_secs: 30,
        context: "https://horizon.stellar.org".to_string(),
    };
    assert!(err.is_retryable());
    assert_eq!(err.log_level(), tracing::Level::WARN);
}

#[test]
fn test_http_request_error_retryable() {
    let err = IndexerError::HttpRequest {
        url: "https://example.com".to_string(),
        status: Some(503),
        error: "Service unavailable".to_string(),
    };
    assert!(err.is_retryable());
}

#[test]
fn test_invalid_config_not_retryable() {
    let err = IndexerError::InvalidConfig {
        field: "database_url".to_string(),
        reason: "Missing".to_string(),
    };
    assert!(!err.is_retryable());
    assert_eq!(err.log_level(), tracing::Level::ERROR);
}

#[test]
fn test_json_parse_error_conversion() {
    let json_err = serde_json::from_str::<serde_json::Value>("invalid json");
    assert!(json_err.is_err());

    let indexer_err: IndexerError = json_err.unwrap_err().into();
    match indexer_err {
        IndexerError::JsonParse { context, error } => {
            assert_eq!(context, "JSON deserialization");
            assert!(!error.is_empty());
        }
        _ => panic!("Expected JsonParse error"),
    }
}

#[test]
fn test_reqwest_timeout_conversion() {
    let err = IndexerError::NetworkTimeout {
        timeout_secs: 30,
        context: "test".to_string(),
    };
    assert!(matches!(err, IndexerError::NetworkTimeout { .. }));
}

#[test]
fn test_numeric_parse_error() {
    let err = IndexerError::NumericParse {
        value: "abc".to_string(),
        expected_type: "u64".to_string(),
    };
    assert!(err.to_string().contains("abc"));
    assert!(err.to_string().contains("u64"));
}

#[test]
fn test_missing_field_error() {
    let err = IndexerError::MissingField {
        field: "asset_code".to_string(),
        context: "Horizon response".to_string(),
    };
    assert!(err.to_string().contains("asset_code"));
    assert!(err.to_string().contains("Horizon response"));
}

#[test]
fn test_stellar_api_error_format() {
    let err = IndexerError::StellarApi {
        endpoint: "/offers".to_string(),
        status: 429,
        message: "Too many requests".to_string(),
    };
    let display = err.to_string();
    assert!(display.contains("/offers"));
    assert!(display.contains("429"));
    assert!(display.contains("Too many requests"));
}

#[test]
fn test_invalid_offer_error() {
    let err = IndexerError::InvalidOffer {
        offer_id: "12345".to_string(),
        reason: "Negative price".to_string(),
    };
    assert!(err.to_string().contains("12345"));
    assert!(err.to_string().contains("Negative price"));
}

#[test]
fn test_error_chain() {
    let sqlx_err = sqlx::Error::RowNotFound;
    let indexer_err: IndexerError = sqlx_err.into();
    assert!(matches!(indexer_err, IndexerError::DatabaseQuery(_)));
}

#[test]
fn test_config_error_conversion() {
    let config_err = config::ConfigError::NotFound("test".to_string());
    let indexer_err: IndexerError = config_err.into();
    assert!(matches!(indexer_err, IndexerError::Config(_)));
}
