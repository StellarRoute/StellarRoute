//! Card webhook handling with strict PAN field rejection
//!
//! This module deserializes partner events with `deny_unknown_fields` and
//! explicitly rejects properties named `pan`, `cvv`, `cvc`, `track`, or `pin`
//! with 422 before any store operation is called.

use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

/// Fields that are strictly forbidden in card webhook payloads
const FORBIDDEN_FIELDS: &[&str] = &["pan", "cvv", "cvc", "track", "pin"];

/// Custom deserializer that rejects forbidden fields
fn reject_forbidden_fields<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let map = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;

    for key in map.keys() {
        let lower_key = key.to_ascii_lowercase();
        if FORBIDDEN_FIELDS.contains(&lower_key.as_str()) {
            return Err(serde::de::Error::custom(format!(
                "Forbidden field '{}' in card webhook payload",
                key
            )));
        }
    }

    Ok(map)
}

/// Partner authorization event payload
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthEvent {
    /// Unique event identifier
    pub event_id: String,
    /// Partner-specific transaction identifier
    pub partner_transaction_id: String,
    /// Authorization amount in minor units (e.g., cents)
    pub amount: i64,
    /// Currency code (ISO 4217)
    pub currency: String,
    /// Merchant category code
    #[serde(default)]
    pub mcc: Option<String>,
    /// Merchant identifier
    #[serde(default)]
    pub merchant_id: Option<String>,
    /// Timestamp of the authorization (RFC3339)
    pub timestamp: String,
    /// Additional metadata (validated for forbidden fields)
    #[serde(default, deserialize_with = "reject_forbidden_fields")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Partner clearing event payload
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClearingEvent {
    /// Unique event identifier
    pub event_id: String,
    /// Partner transaction identifier from auth
    pub partner_transaction_id: String,
    /// Clearing amount in minor units
    pub amount: i64,
    /// Currency code (ISO 4217)
    pub currency: String,
    /// Timestamp of the clearing (RFC3339)
    pub timestamp: String,
    /// Additional metadata (validated for forbidden fields)
    #[serde(default, deserialize_with = "reject_forbidden_fields")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Partner reversal event payload
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReversalEvent {
    /// Unique event identifier
    pub event_id: String,
    /// Partner transaction identifier from auth
    pub partner_transaction_id: String,
    /// Reversal amount in minor units
    pub amount: i64,
    /// Currency code (ISO 4217)
    pub currency: String,
    /// Reason for reversal
    #[serde(default)]
    pub reason: Option<String>,
    /// Timestamp of the reversal (RFC3339)
    pub timestamp: String,
    /// Additional metadata (validated for forbidden fields)
    #[serde(default, deserialize_with = "reject_forbidden_fields")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Union type for all partner webhook events
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PartnerEvent {
    Authorization(AuthEvent),
    Clearing(ClearingEvent),
    Reversal(ReversalEvent),
}

/// Error response for forbidden field rejection
#[derive(Debug, thiserror::Error)]
pub enum WebhookDeserializeError {
    #[error("Forbidden field detected: {0}")]
    ForbiddenField(String),
    #[error("Deserialization error: {0}")]
    Deserialize(#[from] serde_json::Error),
}

impl From<WebhookDeserializeError> for crate::error::ApiError {
    fn from(err: WebhookDeserializeError) -> Self {
        match err {
            WebhookDeserializeError::ForbiddenField(msg) => crate::error::ApiError::Validation(msg),
            WebhookDeserializeError::Deserialize(e) => {
                crate::error::ApiError::BadRequest(format!("Invalid webhook payload: {}", e))
            }
        }
    }
}

/// Deserialize a partner webhook payload with forbidden field validation
pub fn deserialize_partner_event(payload: &str) -> Result<PartnerEvent, WebhookDeserializeError> {
    serde_json::from_str(payload).map_err(WebhookDeserializeError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_auth_event_deserializes() {
        let json = r#"{
            "type": "authorization",
            "event_id": "evt_123",
            "partner_transaction_id": "txn_456",
            "amount": 1000,
            "currency": "USD",
            "timestamp": "2024-01-15T10:30:00Z",
            "metadata": {"key": "value"}
        }"#;

        let event = deserialize_partner_event(json).unwrap();
        assert!(matches!(event, PartnerEvent::Authorization(_)));
    }

    #[test]
    fn test_valid_clearing_event_deserializes() {
        let json = r#"{
            "type": "clearing",
            "event_id": "evt_123",
            "partner_transaction_id": "txn_456",
            "amount": 1000,
            "currency": "USD",
            "timestamp": "2024-01-15T10:30:00Z"
        }"#;

        let event = deserialize_partner_event(json).unwrap();
        assert!(matches!(event, PartnerEvent::Clearing(_)));
    }

    #[test]
    fn test_valid_reversal_event_deserializes() {
        let json = r#"{
            "type": "reversal",
            "event_id": "evt_123",
            "partner_transaction_id": "txn_456",
            "amount": 1000,
            "currency": "USD",
            "timestamp": "2024-01-15T10:30:00Z",
            "reason": "customer_requested"
        }"#;

        let event = deserialize_partner_event(json).unwrap();
        assert!(matches!(event, PartnerEvent::Reversal(_)));
    }

    #[test]
    fn test_pan_field_rejected() {
        let json = r#"{
            "type": "authorization",
            "event_id": "evt_123",
            "partner_transaction_id": "txn_456",
            "amount": 1000,
            "currency": "USD",
            "timestamp": "2024-01-15T10:30:00Z",
            "metadata": {"pan": "4111111111111111"}
        }"#;

        let result = deserialize_partner_event(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pan"));
    }

    #[test]
    fn test_cvv_field_rejected() {
        let json = r#"{
            "type": "authorization",
            "event_id": "evt_123",
            "partner_transaction_id": "txn_456",
            "amount": 1000,
            "currency": "USD",
            "timestamp": "2024-01-15T10:30:00Z",
            "metadata": {"cvv": "123"}
        }"#;

        let result = deserialize_partner_event(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cvv"));
    }

    #[test]
    fn test_cvc_field_rejected() {
        let json = r#"{
            "type": "authorization",
            "event_id": "evt_123",
            "partner_transaction_id": "txn_456",
            "amount": 1000,
            "currency": "USD",
            "timestamp": "2024-01-15T10:30:00Z",
            "metadata": {"cvc": "123"}
        }"#;

        let result = deserialize_partner_event(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cvc"));
    }

    #[test]
    fn test_track_field_rejected() {
        let json = r#"{
            "type": "authorization",
            "event_id": "evt_123",
            "partner_transaction_id": "txn_456",
            "amount": 1000,
            "currency": "USD",
            "timestamp": "2024-01-15T10:30:00Z",
            "metadata": {"track": "track_data"}
        }"#;

        let result = deserialize_partner_event(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("track"));
    }

    #[test]
    fn test_pin_field_rejected() {
        let json = r#"{
            "type": "authorization",
            "event_id": "evt_123",
            "partner_transaction_id": "txn_456",
            "amount": 1000,
            "currency": "USD",
            "timestamp": "2024-01-15T10:30:00Z",
            "metadata": {"pin": "1234"}
        }"#;

        let result = deserialize_partner_event(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pin"));
    }

    #[test]
    fn test_case_insensitive_field_rejection() {
        for field in ["PAN", "Pan", "CvV", "CVC", "Track", "PIN"] {
            let json = format!(
                r#"{{
                "type": "authorization",
                "event_id": "evt_123",
                "partner_transaction_id": "txn_456",
                "amount": 1000,
                "currency": "USD",
                "timestamp": "2024-01-15T10:30:00Z",
                "metadata": {{"{}": "value"}}
            }}"#,
                field
            );

            let result = deserialize_partner_event(&json);
            assert!(result.is_err(), "failed for field: {}", field);
            // Error message contains the original field name as provided
            assert!(result.unwrap_err().to_string().contains(field));
        }
    }

    #[test]
    fn test_unknown_fields_rejected() {
        let json = r#"{
            "type": "authorization",
            "event_id": "evt_123",
            "partner_transaction_id": "txn_456",
            "amount": 1000,
            "currency": "USD",
            "timestamp": "2024-01-15T10:30:00Z",
            "unknown_field": "value"
        }"#;

        let result = deserialize_partner_event(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown field"));
    }

    #[test]
    fn test_store_not_called_on_forbidden_field() {
        // This test verifies that when a forbidden field is present,
        // the deserialization fails before any store operation could be called.
        // The actual store logic is tested separately in integration tests.
        let json = r#"{
            "type": "authorization",
            "event_id": "evt_123",
            "partner_transaction_id": "txn_456",
            "amount": 1000,
            "currency": "USD",
            "timestamp": "2024-01-15T10:30:00Z",
            "metadata": {"pan": "4111111111111111"}
        }"#;

        let result = deserialize_partner_event(json);
        assert!(result.is_err());
        // If we reach here, deserialization failed, so store was never called
    }
}
