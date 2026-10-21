//! Authorization state machine (CARD-15).
//!
//! State transitions:
//! - Pending -> Approved
//! - Pending -> Voided
//! - Pending -> Expired
//! - Approved -> Captured
//! - Approved -> Voided
//! - Approved -> Expired
//!
//! Terminal states: Captured, Voided, Expired.
//! Illegal jumps fail and return an error containing the old state.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationState {
    Pending,
    Approved,
    Captured,
    Voided,
    Expired,
}

impl std::fmt::Display for AuthorizationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Captured => write!(f, "captured"),
            Self::Voided => write!(f, "voided"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationEvent {
    Approve,
    Capture,
    Void,
    Expire,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("illegal state transition from {from:?} to {to:?}")]
pub struct TransitionError {
    pub from: AuthorizationState,
    pub to: AuthorizationState,
}

impl TransitionError {
    pub fn new(from: AuthorizationState, to: AuthorizationState) -> Self {
        Self { from, to }
    }

    /// Returns the old state before the attempted transition.
    pub fn old_state(&self) -> AuthorizationState {
        self.from
    }
}

/// Pure transition function moving from `current` to `target`.
///
/// Returns `Ok(target)` on legal edges, or `Err(TransitionError)` preserving
/// the old state on illegal edges.
pub fn transition(
    current: AuthorizationState,
    target: AuthorizationState,
) -> Result<AuthorizationState, TransitionError> {
    match (current, target) {
        (AuthorizationState::Pending, AuthorizationState::Approved)
        | (AuthorizationState::Pending, AuthorizationState::Voided)
        | (AuthorizationState::Pending, AuthorizationState::Expired)
        | (AuthorizationState::Approved, AuthorizationState::Captured)
        | (AuthorizationState::Approved, AuthorizationState::Voided)
        | (AuthorizationState::Approved, AuthorizationState::Expired) => Ok(target),
        _ => Err(TransitionError::new(current, target)),
    }
}

/// Apply a domain event to an authorization state.
pub fn apply_event(
    current: AuthorizationState,
    event: AuthorizationEvent,
) -> Result<AuthorizationState, TransitionError> {
    let target = match event {
        AuthorizationEvent::Approve => AuthorizationState::Approved,
        AuthorizationEvent::Capture => AuthorizationState::Captured,
        AuthorizationEvent::Void => AuthorizationState::Voided,
        AuthorizationEvent::Expire => AuthorizationState::Expired,
    };
    transition(current, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legal_transitions_from_pending() {
        assert_eq!(
            transition(AuthorizationState::Pending, AuthorizationState::Approved),
            Ok(AuthorizationState::Approved)
        );
        assert_eq!(
            transition(AuthorizationState::Pending, AuthorizationState::Voided),
            Ok(AuthorizationState::Voided)
        );
        assert_eq!(
            transition(AuthorizationState::Pending, AuthorizationState::Expired),
            Ok(AuthorizationState::Expired)
        );
    }

    #[test]
    fn test_legal_transitions_from_approved() {
        assert_eq!(
            transition(AuthorizationState::Approved, AuthorizationState::Captured),
            Ok(AuthorizationState::Captured)
        );
        assert_eq!(
            transition(AuthorizationState::Approved, AuthorizationState::Voided),
            Ok(AuthorizationState::Voided)
        );
        assert_eq!(
            transition(AuthorizationState::Approved, AuthorizationState::Expired),
            Ok(AuthorizationState::Expired)
        );
    }

    #[test]
    fn test_apply_event_legal() {
        assert_eq!(
            apply_event(AuthorizationState::Pending, AuthorizationEvent::Approve),
            Ok(AuthorizationState::Approved)
        );
        assert_eq!(
            apply_event(AuthorizationState::Approved, AuthorizationEvent::Capture),
            Ok(AuthorizationState::Captured)
        );
        assert_eq!(
            apply_event(AuthorizationState::Approved, AuthorizationEvent::Void),
            Ok(AuthorizationState::Voided)
        );
        assert_eq!(
            apply_event(AuthorizationState::Approved, AuthorizationEvent::Expire),
            Ok(AuthorizationState::Expired)
        );
    }

    #[test]
    fn test_illegal_edge_captured_to_pending() {
        let err =
            transition(AuthorizationState::Captured, AuthorizationState::Pending).unwrap_err();
        assert_eq!(err.old_state(), AuthorizationState::Captured);
        assert_eq!(err.from, AuthorizationState::Captured);
        assert_eq!(err.to, AuthorizationState::Pending);
    }

    #[test]
    fn test_illegal_edge_voided_to_approved() {
        let err = transition(AuthorizationState::Voided, AuthorizationState::Approved).unwrap_err();
        assert_eq!(err.old_state(), AuthorizationState::Voided);
        assert_eq!(err.from, AuthorizationState::Voided);
        assert_eq!(err.to, AuthorizationState::Approved);
    }

    #[test]
    fn test_illegal_edge_expired_to_captured() {
        let err =
            transition(AuthorizationState::Expired, AuthorizationState::Captured).unwrap_err();
        assert_eq!(err.old_state(), AuthorizationState::Expired);
        assert_eq!(err.from, AuthorizationState::Expired);
        assert_eq!(err.to, AuthorizationState::Captured);
    }

    #[test]
    fn test_illegal_edge_pending_to_spent_without_approved() {
        // Pending cannot jump directly to Captured (spent)
        let err =
            transition(AuthorizationState::Pending, AuthorizationState::Captured).unwrap_err();
        assert_eq!(err.old_state(), AuthorizationState::Pending);
        assert_eq!(err.from, AuthorizationState::Pending);
        assert_eq!(err.to, AuthorizationState::Captured);
    }

    #[test]
    fn test_terminal_states_reject_all_outgoing_transitions() {
        let terminal_states = [
            AuthorizationState::Captured,
            AuthorizationState::Voided,
            AuthorizationState::Expired,
        ];
        let all_targets = [
            AuthorizationState::Pending,
            AuthorizationState::Approved,
            AuthorizationState::Captured,
            AuthorizationState::Voided,
            AuthorizationState::Expired,
        ];

        for current in terminal_states {
            for target in all_targets {
                let err = transition(current, target).unwrap_err();
                assert_eq!(err.old_state(), current);
            }
        }
    }
}
