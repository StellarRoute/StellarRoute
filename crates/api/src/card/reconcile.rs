//! Pure card authorization/partner-event reconciliation.
//!
//! This module deliberately contains no route, database, kill-switch, or
//! logging behavior. The caller supplies an authorization baseline, the
//! partner events it has accepted, and the current ledger balances. The
//! result is a deterministic list of mismatches suitable for an operator or a
//! later card store adapter.

use std::collections::{BTreeMap, BTreeSet};

/// An authorization that is expected to create a hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardAuthorization {
    pub auth_id: String,
    pub amount_stroops: i64,
}

/// The partner events understood by the card ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardPartnerEventKind {
    /// The authorization baseline event; the authorization record already
    /// represents the hold, so this event is a no-op for reconciliation.
    Authorization,
    /// Capture the locked amount and move it from held to spent.
    Clearing,
    /// Release an uncleared hold.
    Reversal,
    /// Return all or part of a captured amount from spent to available.
    Refund { amount_stroops: i64 },
}

/// A partner event associated with an authorization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardPartnerEvent {
    pub event_id: String,
    pub auth_id: String,
    pub kind: CardPartnerEventKind,
}

/// The aggregate ledger values relevant to reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardLedgerSnapshot {
    pub held_stroops: i64,
    pub spent_stroops: i64,
}

/// A machine-readable reason for a reconciliation mismatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardReconciliationIssue {
    /// The ledger shows a capture that has no corresponding clearing event.
    MissingClearing,
    /// More than one distinct clearing event attempted to capture an auth.
    DoubleCapture,
    /// An event referred to an authorization that was not supplied.
    UnknownAuthorization,
    /// An event was invalid for the current state (for example, a refund before
    /// capture or a reversal after capture).
    InvalidTransition,
    /// A refund attempted to return more than the captured amount.
    RefundExceedsCaptured,
    /// The aggregate held/spent values differ without a more specific event
    /// explanation.
    AggregateDrift,
}

/// A reconciliation mismatch with the authorization IDs involved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardReconciliationMismatch {
    pub auth_ids: Vec<String>,
    pub issues: Vec<CardReconciliationIssue>,
    pub expected_held_stroops: i64,
    pub actual_held_stroops: i64,
    pub expected_spent_stroops: i64,
    pub actual_spent_stroops: i64,
}

#[derive(Clone, Debug)]
struct AuthState {
    amount_stroops: i64,
    held_stroops: i64,
    spent_stroops: i64,
    cleared: bool,
    reversed: bool,
}

/// Reconciles card authorizations and accepted partner events against a ledger.
///
/// Authorization records are the expected initial hold. `Authorization` events
/// are therefore idempotent no-ops; `Clearing` moves the locked amount from
/// held to spent; `Reversal` releases the hold; and `Refund` reduces spent.
/// Duplicate event IDs are ignored, while a second *distinct* clearing event
/// is reported as a double capture. Invalid transitions are reported instead
/// of being silently clamped.
///
/// The ledger is aggregate, so an aggregate drift is attributed to all supplied
/// authorization IDs unless event-level diagnostics identify a narrower set.
/// No I/O, persistence, or mutable global state is used.
pub fn reconcile_card_ledger(
    authorizations: &[CardAuthorization],
    events: &[CardPartnerEvent],
    ledger: CardLedgerSnapshot,
) -> Vec<CardReconciliationMismatch> {
    let mut states = BTreeMap::<String, AuthState>::new();
    let mut issue_ids = BTreeMap::<CardReconciliationIssue, BTreeSet<String>>::new();

    for authorization in authorizations {
        if authorization.auth_id.trim().is_empty() {
            continue;
        }
        if authorization.amount_stroops <= 0 {
            add_issue(
                &mut issue_ids,
                CardReconciliationIssue::InvalidTransition,
                &authorization.auth_id,
            );
            continue;
        }
        if states.contains_key(&authorization.auth_id) {
            add_issue(
                &mut issue_ids,
                CardReconciliationIssue::InvalidTransition,
                &authorization.auth_id,
            );
            continue;
        }

        states.insert(
            authorization.auth_id.clone(),
            AuthState {
                amount_stroops: authorization.amount_stroops,
                held_stroops: authorization.amount_stroops,
                spent_stroops: 0,
                cleared: false,
                reversed: false,
            },
        );
    }

    let mut seen_event_ids = BTreeSet::new();
    for event in events {
        if !seen_event_ids.insert(event.event_id.clone()) {
            // CARD-26 makes duplicate event IDs a no-op success.
            continue;
        }

        let Some(state) = states.get_mut(&event.auth_id) else {
            add_issue(
                &mut issue_ids,
                CardReconciliationIssue::UnknownAuthorization,
                &event.auth_id,
            );
            continue;
        };

        match event.kind {
            CardPartnerEventKind::Authorization => {
                // The authorization record is the canonical initial hold.
            }
            CardPartnerEventKind::Clearing => {
                if state.cleared {
                    add_issue(
                        &mut issue_ids,
                        CardReconciliationIssue::DoubleCapture,
                        &event.auth_id,
                    );
                    continue;
                }
                if state.reversed || state.held_stroops < state.amount_stroops {
                    add_issue(
                        &mut issue_ids,
                        CardReconciliationIssue::InvalidTransition,
                        &event.auth_id,
                    );
                    continue;
                }
                state.held_stroops -= state.amount_stroops;
                state.spent_stroops = state
                    .spent_stroops
                    .checked_add(state.amount_stroops)
                    .unwrap_or_else(|| {
                        add_issue(
                            &mut issue_ids,
                            CardReconciliationIssue::InvalidTransition,
                            &event.auth_id,
                        );
                        state.spent_stroops
                    });
                state.cleared = true;
            }
            CardPartnerEventKind::Reversal => {
                if state.cleared || state.reversed || state.held_stroops < state.amount_stroops {
                    add_issue(
                        &mut issue_ids,
                        CardReconciliationIssue::InvalidTransition,
                        &event.auth_id,
                    );
                    continue;
                }
                state.held_stroops = 0;
                state.reversed = true;
            }
            CardPartnerEventKind::Refund { amount_stroops } => {
                if amount_stroops <= 0 {
                    add_issue(
                        &mut issue_ids,
                        CardReconciliationIssue::InvalidTransition,
                        &event.auth_id,
                    );
                    continue;
                }
                if state.spent_stroops < amount_stroops {
                    add_issue(
                        &mut issue_ids,
                        CardReconciliationIssue::RefundExceedsCaptured,
                        &event.auth_id,
                    );
                    continue;
                }
                state.spent_stroops -= amount_stroops;
            }
        }
    }

    let (mut expected_held, mut expected_spent) = (0_i64, 0_i64);
    for (auth_id, state) in &states {
        expected_held = expected_held
            .checked_add(state.held_stroops)
            .unwrap_or_else(|| {
                add_issue(
                    &mut issue_ids,
                    CardReconciliationIssue::InvalidTransition,
                    auth_id,
                );
                expected_held
            });
        expected_spent = expected_spent
            .checked_add(state.spent_stroops)
            .unwrap_or_else(|| {
                add_issue(
                    &mut issue_ids,
                    CardReconciliationIssue::InvalidTransition,
                    auth_id,
                );
                expected_spent
            });
    }

    let has_event_issue = !issue_ids.is_empty();
    let aggregate_drift =
        expected_held != ledger.held_stroops || expected_spent != ledger.spent_stroops;
    if !has_event_issue && !aggregate_drift {
        return Vec::new();
    }

    let mut issues = BTreeSet::new();
    let mut auth_ids = BTreeSet::new();
    for (issue, ids) in &issue_ids {
        issues.insert(*issue);
        auth_ids.extend(ids.iter().cloned());
    }

    if aggregate_drift {
        if issues.is_empty() {
            issues.insert(CardReconciliationIssue::AggregateDrift);
        } else if !issues.contains(&CardReconciliationIssue::DoubleCapture)
            && ledger.spent_stroops > expected_spent
            && ledger.held_stroops < expected_held
        {
            issues.insert(CardReconciliationIssue::MissingClearing);
        }
        if auth_ids.is_empty() {
            auth_ids.extend(states.keys().cloned());
        }
    }

    vec![CardReconciliationMismatch {
        auth_ids: auth_ids.into_iter().collect(),
        issues: issues.into_iter().collect(),
        expected_held_stroops: expected_held,
        actual_held_stroops: ledger.held_stroops,
        expected_spent_stroops: expected_spent,
        actual_spent_stroops: ledger.spent_stroops,
    }]
}

fn add_issue(
    issues: &mut BTreeMap<CardReconciliationIssue, BTreeSet<String>>,
    issue: CardReconciliationIssue,
    auth_id: &str,
) {
    issues.entry(issue).or_default().insert(auth_id.to_owned());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authorization(id: &str, amount: i64) -> CardAuthorization {
        CardAuthorization {
            auth_id: id.to_owned(),
            amount_stroops: amount,
        }
    }

    fn event(id: &str, auth_id: &str, kind: CardPartnerEventKind) -> CardPartnerEvent {
        CardPartnerEvent {
            event_id: id.to_owned(),
            auth_id: auth_id.to_owned(),
            kind,
        }
    }

    #[test]
    fn reports_missing_clearing_with_auth_id() {
        let result = reconcile_card_ledger(
            &[authorization("auth-1", 100)],
            &[],
            CardLedgerSnapshot {
                held_stroops: 0,
                spent_stroops: 100,
            },
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].auth_ids, vec!["auth-1"]);
        assert!(result[0]
            .issues
            .contains(&CardReconciliationIssue::MissingClearing));
    }

    #[test]
    fn balanced_set_returns_no_mismatches() {
        let result = reconcile_card_ledger(
            &[authorization("auth-1", 100)],
            &[event("event-1", "auth-1", CardPartnerEventKind::Clearing)],
            CardLedgerSnapshot {
                held_stroops: 0,
                spent_stroops: 100,
            },
        );

        assert!(result.is_empty());
    }

    #[test]
    fn reports_double_capture_with_auth_id() {
        let result = reconcile_card_ledger(
            &[authorization("auth-1", 100)],
            &[
                event("event-1", "auth-1", CardPartnerEventKind::Clearing),
                event("event-2", "auth-1", CardPartnerEventKind::Clearing),
            ],
            CardLedgerSnapshot {
                held_stroops: 0,
                spent_stroops: 200,
            },
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].auth_ids, vec!["auth-1"]);
        assert!(result[0]
            .issues
            .contains(&CardReconciliationIssue::DoubleCapture));
        assert_eq!(result[0].expected_spent_stroops, 100);
        assert_eq!(result[0].actual_spent_stroops, 200);
    }

    #[test]
    fn duplicate_event_id_is_a_no_op() {
        let clearing = event("event-1", "auth-1", CardPartnerEventKind::Clearing);
        let result = reconcile_card_ledger(
            &[authorization("auth-1", 100)],
            &[clearing.clone(), clearing],
            CardLedgerSnapshot {
                held_stroops: 0,
                spent_stroops: 100,
            },
        );

        assert!(result.is_empty());
    }

    #[test]
    fn refund_above_capture_is_reported_without_underflow() {
        let result = reconcile_card_ledger(
            &[authorization("auth-1", 100)],
            &[
                event("event-1", "auth-1", CardPartnerEventKind::Clearing),
                event(
                    "event-2",
                    "auth-1",
                    CardPartnerEventKind::Refund {
                        amount_stroops: 101,
                    },
                ),
            ],
            CardLedgerSnapshot {
                held_stroops: 0,
                spent_stroops: 100,
            },
        );

        assert_eq!(result.len(), 1);
        assert!(result[0]
            .issues
            .contains(&CardReconciliationIssue::RefundExceedsCaptured));
        assert_eq!(result[0].expected_spent_stroops, 100);
    }
}

/// Short alias for callers that already use the card module namespace.
pub use reconcile_card_ledger as reconcile;
