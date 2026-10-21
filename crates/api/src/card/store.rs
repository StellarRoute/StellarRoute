//! Card store: applications, authorizations, USDC holds, and partner events.
//!
//! In-memory implementation for CARD-14. Nothing here is touched while `CARD_ENABLED` is off.

use std::collections::HashMap;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Authorization lifecycle states used by the recording path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationState {
    Pending,
    Approved,
    Captured,
    Reversed,
    Refunded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationRecord {
    pub application_id: String,
    pub name: String,
    pub country: String,
    pub currency: String,
    pub monthly_limit_usdc: u64,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationRecord {
    pub authorization_id: String,
    pub state: AuthorizationState,
    /// Locked USDC amount in stroops (7 decimals).
    pub amount_stroops: i64,
    /// Amount captured from the hold so far.
    pub spent_stroops: i64,
    pub fiat_amount: i64,
    pub fiat_currency: String,
    pub rate: f64,
    pub rate_locked_at: i64,
    pub tx_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartnerEventRecord {
    pub event_id: String,
    pub event_type: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    #[error("record with id '{0}' not found")]
    UnknownId(String),
    #[error("record with id '{0}' already exists")]
    AlreadyExists(String),
    #[error("authorization already approved")]
    AlreadyApproved,
    #[error("tx hash already used for another authorization")]
    TxAlreadyUsed,
    /// Authorization record is not known.
    AuthorizationNotFound,
    /// No active hold exists for this authorization.
    HoldNotFound,
    /// Event amount exceeds the current hold or captured amount.
    AmountExceedsAvailable,
    /// Refund exceeded the amount already captured.
    RefundExceedsCaptured,
    /// Webhook fiat amount differs from the locked authorization value by more than one minor unit.
    FiatAmountMismatch,
    /// Unsupported partner event type.
    UnsupportedEventType,
}

pub trait CardStore: Send + Sync {
    // Applications (CARD-14)
    fn insert_application(&self, app: ApplicationRecord) -> Result<ApplicationRecord, StoreError>;
    fn get_application(&self, application_id: &str) -> Option<ApplicationRecord>;
    fn update_application(&self, app: ApplicationRecord) -> Result<ApplicationRecord, StoreError>;

    // Authorizations (CARD-14)
    fn insert_authorization(
        &self,
        auth: AuthorizationRecord,
    ) -> Result<AuthorizationRecord, StoreError>;
    fn get_authorization(&self, authorization_id: &str) -> Option<AuthorizationRecord>;
    fn update_authorization(
        &self,
        auth: AuthorizationRecord,
    ) -> Result<AuthorizationRecord, StoreError>;

    // Backwards-compatible authorization helper
    fn approve_and_hold(
        &self,
        authorization_id: &str,
        tx_hash: &str,
        amount_stroops: i64,
    ) -> Result<AuthorizationRecord, StoreError> {
        self.approve_and_hold_with_fx(
            authorization_id,
            tx_hash,
            amount_stroops,
            0,
            "",
            0.0,
            0,
        )
    }

    fn approve_and_hold_with_fx(
        &self,
        authorization_id: &str,
        tx_hash: &str,
        amount_stroops: i64,
        fiat_amount: i64,
        fiat_currency: &str,
        rate: f64,
        rate_locked_at: i64,
    ) -> Result<AuthorizationRecord, StoreError>;
    /// Capture a cleared amount from the current hold.
    fn capture_hold(&self, authorization_id: &str, amount_stroops: i64) -> Result<AuthorizationRecord, StoreError>;
    /// Release an existing hold while keeping the authorization record intact.
    fn reverse_hold(&self, authorization_id: &str) -> Result<AuthorizationRecord, StoreError>;
    /// Refund already captured funds back to available balance up to the captured amount.
    fn refund_hold(&self, authorization_id: &str, amount_stroops: i64) -> Result<AuthorizationRecord, StoreError>;
    fn list_authorizations(&self) -> Vec<AuthorizationRecord>;
    fn authorization(&self, authorization_id: &str) -> Option<AuthorizationRecord>;
    /// Current held amount in stroops for an authorization (0 if none).
    fn held_stroops(&self, authorization_id: &str) -> i64;

    // Partner events
    fn insert_partner_event(&self, event: PartnerEventRecord) -> bool;
    fn partner_event(&self, event_id: &str) -> Option<PartnerEventRecord>;
    fn partner_event_count(&self) -> usize;
}

#[derive(Default)]
struct Inner {
    applications: HashMap<String, ApplicationRecord>,
    authorizations: HashMap<String, AuthorizationRecord>,
    tx_to_auth: HashMap<String, String>,
    holds: HashMap<String, i64>,
    events: HashMap<String, PartnerEventRecord>,
}

#[derive(Default)]
pub struct InMemoryCardStore {
    inner: Mutex<Inner>,
}

impl CardStore for InMemoryCardStore {
    fn insert_application(&self, app: ApplicationRecord) -> Result<ApplicationRecord, StoreError> {
        let mut inner = self.inner.lock();
        if inner.applications.contains_key(&app.application_id) {
            return Err(StoreError::AlreadyExists(app.application_id));
        }
        inner
            .applications
            .insert(app.application_id.clone(), app.clone());
        Ok(app)
    }

    fn get_application(&self, application_id: &str) -> Option<ApplicationRecord> {
        self.inner.lock().applications.get(application_id).cloned()
    }

    fn update_application(&self, app: ApplicationRecord) -> Result<ApplicationRecord, StoreError> {
        let mut inner = self.inner.lock();
        if !inner.applications.contains_key(&app.application_id) {
            return Err(StoreError::UnknownId(app.application_id));
        }
        inner
            .applications
            .insert(app.application_id.clone(), app.clone());
        Ok(app)
    }

    fn insert_authorization(
        &self,
        auth: AuthorizationRecord,
    ) -> Result<AuthorizationRecord, StoreError> {
        let mut inner = self.inner.lock();
        if inner.authorizations.contains_key(&auth.authorization_id) {
            return Err(StoreError::AlreadyExists(auth.authorization_id));
        }
        if !auth.tx_hash.is_empty() {
            if let Some(existing) = inner.tx_to_auth.get(&auth.tx_hash) {
                if existing != &auth.authorization_id {
                    return Err(StoreError::TxAlreadyUsed);
                }
            }
            inner
                .tx_to_auth
                .insert(auth.tx_hash.clone(), auth.authorization_id.clone());
        }
        if auth.amount_stroops > 0 {
            inner
                .holds
                .insert(auth.authorization_id.clone(), auth.amount_stroops);
        }
        inner
            .authorizations
            .insert(auth.authorization_id.clone(), auth.clone());
        Ok(auth)
    }

    fn get_authorization(&self, authorization_id: &str) -> Option<AuthorizationRecord> {
        self.inner
            .lock()
            .authorizations
            .get(authorization_id)
            .cloned()
    }

    fn update_authorization(
        &self,
        auth: AuthorizationRecord,
    ) -> Result<AuthorizationRecord, StoreError> {
        let mut inner = self.inner.lock();
        if !inner.authorizations.contains_key(&auth.authorization_id) {
            return Err(StoreError::UnknownId(auth.authorization_id));
        }
        inner
            .authorizations
            .insert(auth.authorization_id.clone(), auth.clone());
        Ok(auth)
    }

    fn approve_and_hold_with_fx(
        &self,
        authorization_id: &str,
        tx_hash: &str,
        amount_stroops: i64,
        fiat_amount: i64,
        fiat_currency: &str,
        rate: f64,
        rate_locked_at: i64,
    ) -> Result<AuthorizationRecord, StoreError> {
        let mut inner = self.inner.lock();
        if let Some(existing) = inner.tx_to_auth.get(tx_hash) {
            if existing != authorization_id {
                return Err(StoreError::TxAlreadyUsed);
            }
        }
        if matches!(
            inner.authorizations.get(authorization_id),
            Some(r) if r.state == AuthorizationState::Approved
        ) {
            return Err(StoreError::AlreadyApproved);
        }
        let record = AuthorizationRecord {
            authorization_id: authorization_id.to_string(),
            state: AuthorizationState::Approved,
            amount_stroops,
            spent_stroops: 0,
            fiat_amount,
            fiat_currency: fiat_currency.to_string(),
            rate,
            rate_locked_at,
            tx_hash: tx_hash.to_string(),
        };
        inner
            .authorizations
            .insert(authorization_id.to_string(), record.clone());
        inner
            .tx_to_auth
            .insert(tx_hash.to_string(), authorization_id.to_string());
        inner
            .holds
            .insert(authorization_id.to_string(), amount_stroops);
        Ok(record)
    }

    fn capture_hold(&self, authorization_id: &str, amount_stroops: i64) -> Result<AuthorizationRecord, StoreError> {
        let mut inner = self.inner.lock();
        let mut record = inner
            .authorizations
            .get(authorization_id)
            .cloned()
            .ok_or(StoreError::AuthorizationNotFound)?;

        let current_held = inner.holds.get(authorization_id).copied().unwrap_or(record.amount_stroops);
        if current_held < amount_stroops {
            return Err(StoreError::AmountExceedsAvailable);
        }

        let new_spent = record.spent_stroops + amount_stroops;
        let new_held = current_held - amount_stroops;
        record.spent_stroops = new_spent;
        record.state = AuthorizationState::Captured;
        inner.holds.insert(authorization_id.to_string(), new_held);
        inner.authorizations.insert(authorization_id.to_string(), record.clone());
        Ok(record)
    }

    fn reverse_hold(&self, authorization_id: &str) -> Result<AuthorizationRecord, StoreError> {
        let mut inner = self.inner.lock();
        let mut record = inner
            .authorizations
            .get(authorization_id)
            .cloned()
            .ok_or(StoreError::AuthorizationNotFound)?;

        let held = inner.holds.get(authorization_id).copied().unwrap_or(record.amount_stroops);
        if held == 0 {
            return Err(StoreError::HoldNotFound);
        }

        inner.holds.remove(authorization_id);
        record.amount_stroops = 0;
        record.spent_stroops = 0;
        record.state = AuthorizationState::Reversed;
        inner.authorizations.insert(authorization_id.to_string(), record.clone());
        Ok(record)
    }

    fn refund_hold(&self, authorization_id: &str, amount_stroops: i64) -> Result<AuthorizationRecord, StoreError> {
        let mut inner = self.inner.lock();
        let mut record = inner
            .authorizations
            .get(authorization_id)
            .cloned()
            .ok_or(StoreError::AuthorizationNotFound)?;

        if amount_stroops > record.spent_stroops {
            return Err(StoreError::RefundExceedsCaptured);
        }

        record.spent_stroops -= amount_stroops;
        let hold = inner.holds.get(authorization_id).copied().unwrap_or(record.amount_stroops);
        inner.holds.insert(authorization_id.to_string(), hold + amount_stroops);
        record.state = AuthorizationState::Refunded;
        inner.authorizations.insert(authorization_id.to_string(), record.clone());
        Ok(record)
    }

    fn list_authorizations(&self) -> Vec<AuthorizationRecord> {
        let mut inner = self.inner.lock();
        let mut records: Vec<_> = inner.authorizations.values().cloned().collect();
        records.sort_by(|a, b| a.authorization_id.cmp(&b.authorization_id));
        records
    }

    fn authorization(&self, authorization_id: &str) -> Option<AuthorizationRecord> {
        self.get_authorization(authorization_id)
    }

    fn held_stroops(&self, authorization_id: &str) -> i64 {
        self.inner
            .lock()
            .holds
            .get(authorization_id)
            .copied()
            .unwrap_or(0)
    }

    fn insert_partner_event(&self, event: PartnerEventRecord) -> bool {
        let mut inner = self.inner.lock();
        if inner.events.contains_key(&event.event_id) {
            return false;
        }
        inner.events.insert(event.event_id.clone(), event);
        true
    }

    fn partner_event(&self, event_id: &str) -> Option<PartnerEventRecord> {
        self.inner.lock().events.get(event_id).cloned()
    }

    fn partner_event_count(&self) -> usize {
        self.inner.lock().events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_insert_then_get_returns_row() {
        let store = InMemoryCardStore::default();
        let app = ApplicationRecord {
            application_id: "app-1".into(),
            name: "Alice Smith".into(),
            country: "US".into(),
            currency: "USD".into(),
            monthly_limit_usdc: 1500,
            status: "draft".into(),
        };
        assert_eq!(store.get_application("app-1"), None);
        assert_eq!(store.insert_application(app.clone()), Ok(app.clone()));
        assert_eq!(store.get_application("app-1"), Some(app));
    }

    #[test]
    fn test_application_update_unknown_id_errors() {
        let store = InMemoryCardStore::default();
        let app = ApplicationRecord {
            application_id: "unknown-app".into(),
            name: "Bob".into(),
            country: "GB".into(),
            currency: "GBP".into(),
            monthly_limit_usdc: 2000,
            status: "draft".into(),
        };
        assert_eq!(
            store.update_application(app),
            Err(StoreError::UnknownId("unknown-app".into()))
        );
    }

    #[test]
    fn test_application_update_success() {
        let store = InMemoryCardStore::default();
        let mut app = ApplicationRecord {
            application_id: "app-2".into(),
            name: "Carol".into(),
            country: "FR".into(),
            currency: "EUR".into(),
            monthly_limit_usdc: 1000,
            status: "draft".into(),
        };
        store.insert_application(app.clone()).unwrap();
        app.status = "approved".into();
        app.monthly_limit_usdc = 2500;
        assert_eq!(store.update_application(app.clone()), Ok(app.clone()));
        assert_eq!(store.get_application("app-2"), Some(app));
    }

    #[test]
    fn test_authorization_insert_then_get_returns_row() {
        let store = InMemoryCardStore::default();
        let auth = AuthorizationRecord {
            authorization_id: "auth-1".into(),
            state: AuthorizationState::Pending,
            amount_stroops: 50_000_000,
            tx_hash: "tx_hash_1".into(),
        };
        assert_eq!(store.get_authorization("auth-1"), None);
        assert_eq!(store.insert_authorization(auth.clone()), Ok(auth.clone()));
        assert_eq!(store.get_authorization("auth-1"), Some(auth));
    }

    #[test]
    fn test_authorization_update_unknown_id_errors() {
        let store = InMemoryCardStore::default();
        let auth = AuthorizationRecord {
            authorization_id: "nonexistent".into(),
            state: AuthorizationState::Approved,
            amount_stroops: 100,
            tx_hash: "hash".into(),
        };
        assert_eq!(
            store.update_authorization(auth),
            Err(StoreError::UnknownId("nonexistent".into()))
        );
    }

    #[test]
    fn test_authorization_update_success() {
        let store = InMemoryCardStore::default();
        let mut auth = AuthorizationRecord {
            authorization_id: "auth-2".into(),
            state: AuthorizationState::Pending,
            amount_stroops: 1000,
            tx_hash: "tx-2".into(),
        };
        store.insert_authorization(auth.clone()).unwrap();
        auth.state = AuthorizationState::Approved;
        assert_eq!(store.update_authorization(auth.clone()), Ok(auth.clone()));
        assert_eq!(store.get_authorization("auth-2"), Some(auth));
    }

    #[test]
    fn approve_holds_amount_and_rejects_tx_reuse() {
        let store = InMemoryCardStore::default();
        store.approve_and_hold("auth-1", "aa", 100).unwrap();
        assert_eq!(store.held_stroops("auth-1"), 100);
        assert_eq!(
            store.approve_and_hold("auth-2", "aa", 100),
            Err(StoreError::TxAlreadyUsed)
        );
        assert_eq!(
            store.approve_and_hold("auth-1", "aa", 100),
            Err(StoreError::AlreadyApproved)
        );
        assert_eq!(store.held_stroops("auth-2"), 0);
    }

    #[test]
    fn partner_events_are_idempotent_by_id() {
        let store = InMemoryCardStore::default();
        let ev = PartnerEventRecord {
            event_id: "evt-1".into(),
            event_type: None,
            payload: serde_json::Value::Null,
        };
        assert!(store.insert_partner_event(ev.clone()));
        assert!(!store.insert_partner_event(ev));
        assert_eq!(store.partner_event_count(), 1);
    }

    #[test]
    fn locked_rate_is_preserved_on_authorization() {
        let store = InMemoryCardStore::default();
        let record = store
            .approve_and_hold_with_fx("auth-1", "tx-1", 100, 2500, "USD", 1.25, 1700000000)
            .unwrap();

        assert_eq!(record.fiat_amount, 2500);
        assert_eq!(record.fiat_currency, "USD");
        assert_eq!(record.rate, 1.25);
        assert_eq!(record.rate_locked_at, 1700000000);
    }

    #[test]
    fn clearing_increases_spent_and_decreases_held() {
        let store = InMemoryCardStore::default();
        store.approve_and_hold("auth-1", "tx-1", 100).unwrap();

        let record = store.capture_hold("auth-1", 100).unwrap();
        assert_eq!(record.spent_stroops, 100);
        assert_eq!(store.held_stroops("auth-1"), 0);
    }

    #[test]
    fn refund_above_captured_fails() {
        let store = InMemoryCardStore::default();
        store.approve_and_hold("auth-1", "tx-1", 100).unwrap();
        store.capture_hold("auth-1", 100).unwrap();

        assert_eq!(
            store.refund_hold("auth-1", 101),
            Err(StoreError::RefundExceedsCaptured)
        );
        assert_eq!(store.authorization("auth-1").unwrap().spent_stroops, 100);
    }
}
