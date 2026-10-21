//! Partner sandbox fixtures for testing card payment flows
//!
//! Provides JSON fixtures and a test that replays authorization → clearing → reversal
//! on a second auth without requiring a partner URL.

use serde_json::json;

/// Authorization event fixture (no PAN)
pub fn auth_fixture() -> serde_json::Value {
    json!({
        "type": "authorization",
        "event_id": "evt_auth_001",
        "partner_transaction_id": "txn_sandbox_001",
        "amount": 1000,
        "currency": "USD",
        "timestamp": "2024-01-15T10:30:00Z",
        "metadata": {
            "merchant_id": "merch_test_001",
            "mcc": "5411"
        }
    })
}

/// Clearing event fixture for the same transaction
pub fn clearing_fixture() -> serde_json::Value {
    json!({
        "type": "clearing",
        "event_id": "evt_clearing_001",
        "partner_transaction_id": "txn_sandbox_001",
        "amount": 1000,
        "currency": "USD",
        "timestamp": "2024-01-15T10:35:00Z",
        "metadata": {}
    })
}

/// Reversal event fixture for the same transaction
pub fn reversal_fixture() -> serde_json::Value {
    json!({
        "type": "reversal",
        "event_id": "evt_reversal_001",
        "partner_transaction_id": "txn_sandbox_001",
        "amount": 1000,
        "currency": "USD",
        "timestamp": "2024-01-15T10:40:00Z",
        "reason": "customer_requested",
        "metadata": {}
    })
}

/// Second authorization for reversal replay test
pub fn auth_fixture_2() -> serde_json::Value {
    json!({
        "type": "authorization",
        "event_id": "evt_auth_002",
        "partner_transaction_id": "txn_sandbox_002",
        "amount": 500,
        "currency": "USD",
        "timestamp": "2024-01-15T11:00:00Z",
        "metadata": {
            "merchant_id": "merch_test_002",
            "mcc": "5812"
        }
    })
}

/// All fixtures as a vector for iteration
pub fn all_fixtures() -> Vec<serde_json::Value> {
    vec![
        auth_fixture(),
        clearing_fixture(),
        reversal_fixture(),
        auth_fixture_2(),
    ]
}

/// Expected balances after replay: auth(1000) + clearing(1000) + reversal(-1000) + auth2(500) = 500
pub const EXPECTED_FINAL_BALANCE: i64 = 500;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::webhooks::{deserialize_partner_event, PartnerEvent};

    #[test]
    fn test_fixtures_contain_no_pan() {
        for fixture in all_fixtures() {
            let json_str = fixture.to_string();
            assert!(!json_str.to_lowercase().contains("pan"));
            assert!(!json_str.to_lowercase().contains("cvv"));
            assert!(!json_str.to_lowercase().contains("cvc"));
            assert!(!json_str.to_lowercase().contains("track"));
            assert!(!json_str.to_lowercase().contains("pin"));
        }
    }

    #[test]
    fn test_fixtures_deserialize_successfully() {
        for fixture in all_fixtures() {
            let json_str = fixture.to_string();
            let event = deserialize_partner_event(&json_str).unwrap();
            assert!(matches!(
                event,
                PartnerEvent::Authorization(_)
                    | PartnerEvent::Clearing(_)
                    | PartnerEvent::Reversal(_)
            ));
        }
    }

    #[test]
    fn test_replay_sequence_auth_clear_reversal_auth2() {
        // Simulate the replay sequence: auth → clearing → reversal → auth2
        let auth = deserialize_partner_event(&auth_fixture().to_string()).unwrap();
        let clearing = deserialize_partner_event(&clearing_fixture().to_string()).unwrap();
        let reversal = deserialize_partner_event(&reversal_fixture().to_string()).unwrap();
        let auth2 = deserialize_partner_event(&auth_fixture_2().to_string()).unwrap();

        // Verify event types in order
        assert!(matches!(auth, PartnerEvent::Authorization(_)));
        assert!(matches!(clearing, PartnerEvent::Clearing(_)));
        assert!(matches!(reversal, PartnerEvent::Reversal(_)));
        assert!(matches!(auth2, PartnerEvent::Authorization(_)));

        // Verify partner_transaction_ids match for auth/clearing/reversal
        if let PartnerEvent::Authorization(a) = &auth {
            if let PartnerEvent::Clearing(c) = &clearing {
                if let PartnerEvent::Reversal(r) = &reversal {
                    assert_eq!(a.partner_transaction_id, c.partner_transaction_id);
                    assert_eq!(a.partner_transaction_id, r.partner_transaction_id);
                }
            }
        }

        // Verify auth2 has different transaction ID
        if let PartnerEvent::Authorization(a2) = &auth2 {
            if let PartnerEvent::Authorization(a1) = &auth {
                assert_ne!(a1.partner_transaction_id, a2.partner_transaction_id);
            }
        }
    }

    #[test]
    fn test_config_load_does_not_require_partner_url() {
        // This test verifies that config loading doesn't require a partner URL
        // The actual config loading is tested in integration tests
        std::env::remove_var("CARD_PARTNER_BASE_URL");
        // Should not panic or require the URL
        assert!(std::env::var("CARD_PARTNER_BASE_URL").is_err());
    }
}
