//! Card store: authorizations, USDC holds, and partner events.
//!
//! In-memory only. Nothing here is touched while `CARD_ENABLED` is off.

use std::collections::HashMap;

use parking_lot::Mutex;
use serde::Serialize;

/// Authorization lifecycle states used by the recording path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationState {
    Pending,
    Approved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorizationRecord {
    pub authorization_id: String,
    pub state: AuthorizationState,
    /// Held USDC amount in stroops (7 decimals).
    pub amount_stroops: i64,
    pub tx_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PartnerEventRecord {
    pub event_id: String,
    pub event_type: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// The authorization id is already approved.
    AlreadyApproved,
    /// The tx hash already approved a different authorization.
    TxAlreadyUsed,
}

pub trait CardStore: Send + Sync {
    /// Mark an authorization approved and hold `amount_stroops` in one step.
    fn approve_and_hold(
        &self,
        authorization_id: &str,
        tx_hash: &str,
        amount_stroops: i64,
    ) -> Result<AuthorizationRecord, StoreError>;
    fn authorization(&self, authorization_id: &str) -> Option<AuthorizationRecord>;
    /// Current held amount in stroops for an authorization (0 if none).
    fn held_stroops(&self, authorization_id: &str) -> i64;
    /// Store a partner event. Returns `false` if the id was already stored.
    fn insert_partner_event(&self, event: PartnerEventRecord) -> bool;
    fn partner_event(&self, event_id: &str) -> Option<PartnerEventRecord>;
    fn partner_event_count(&self) -> usize;
}

#[derive(Default)]
struct Inner {
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
    fn approve_and_hold(
        &self,
        authorization_id: &str,
        tx_hash: &str,
        amount_stroops: i64,
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

    fn authorization(&self, authorization_id: &str) -> Option<AuthorizationRecord> {
        self.inner
            .lock()
            .authorizations
            .get(authorization_id)
            .cloned()
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
}
