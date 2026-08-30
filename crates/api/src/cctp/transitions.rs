//! Allowed CCTP transfer saga state transitions (frozen contract enum).

use crate::models::v2_cctp::CctpTransferStatus;

/// Returns true when `from` may transition to `to` (including idempotent same-state).
pub fn is_allowed_transition(from: CctpTransferStatus, to: CctpTransferStatus) -> bool {
    if from == to {
        return true;
    }

    use CctpTransferStatus::*;
    matches!(
        (from, to),
        (Created, BurnPrepared)
            | (Created, Cancelled)
            | (Created, ProviderKilled)
            | (BurnPrepared, BurnSubmitted)
            | (BurnPrepared, AwaitingAttestation)
            | (BurnPrepared, Cancelled)
            | (BurnPrepared, ProviderKilled)
            | (BurnSubmitted, AwaitingAttestation)
            | (BurnSubmitted, ProviderKilled)
            | (AwaitingAttestation, AttestationReady)
            | (AwaitingAttestation, AttestationFailed)
            | (AwaitingAttestation, ProviderKilled)
            | (AttestationFailed, AwaitingAttestation)
            | (AttestationReady, MintPrepared)
            | (AttestationReady, ProviderKilled)
            | (MintPrepared, MintSubmitted)
            | (MintPrepared, MintFailedRetryable)
            | (MintPrepared, ProviderKilled)
            | (MintSubmitted, Completed)
            | (MintSubmitted, MintFailedRetryable)
            | (MintSubmitted, ProviderKilled)
            | (MintFailedRetryable, MintPrepared)
            | (MintFailedRetryable, ProviderKilled)
    )
}

/// Terminal states are immutable except idempotent self-transitions.
pub fn is_terminal(status: CctpTransferStatus) -> bool {
    matches!(
        status,
        CctpTransferStatus::Completed
            | CctpTransferStatus::Cancelled
            | CctpTransferStatus::ProviderKilled
    )
}

/// Recoverable non-terminal failure — reattest may return to awaiting_attestation.
pub fn is_recoverable_failure(status: CctpTransferStatus) -> bool {
    status == CctpTransferStatus::AttestationFailed
}

/// Cancellation allowed only before source burn is submitted.
pub fn can_cancel(status: CctpTransferStatus) -> bool {
    matches!(
        status,
        CctpTransferStatus::Created | CctpTransferStatus::BurnPrepared
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::v2_cctp::CctpTransferStatus as S;

    #[test]
    fn attestation_failed_is_recoverable_not_terminal() {
        assert!(!is_terminal(S::AttestationFailed));
        assert!(is_recoverable_failure(S::AttestationFailed));
        assert!(is_allowed_transition(
            S::AttestationFailed,
            S::AwaitingAttestation
        ));
    }

    #[test]
    fn terminal_states_immutable() {
        for terminal in [S::Completed, S::Cancelled, S::ProviderKilled] {
            assert!(is_terminal(terminal));
            assert!(!is_allowed_transition(terminal, S::AwaitingAttestation));
        }
    }
}
