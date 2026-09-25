//! Card ledger for tracking hold, capture, and release of USDC amounts.
//!
//! This module implements an in-memory ledger that tracks `{ available, held, spent }`
//! balances. It supports card operations: hold moves available to held, capture moves
//! held to spent, and release moves held to available. Over-hold and over-capture
//! return errors.

use thiserror::Error;

/// Errors that can occur during ledger operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum LedgerError {
    #[error("Over-hold: cannot hold {amount} when only {available} is available")]
    OverHold { amount: f64, available: f64 },
    #[error("Over-capture: cannot capture {amount} when only {held} is held")]
    OverCapture { amount: f64, held: f64 },
}

/// Result type for ledger operations.
pub type LedgerResult<T> = std::result::Result<T, LedgerError>;

/// Card ledger tracking available, held, and spent USDC amounts.
#[derive(Debug, Clone)]
pub struct CardLedger {
    /// Amount currently available for hold.
    pub available: f64,
    /// Amount currently held (authorized but not yet captured).
    pub held: f64,
    /// Amount already captured (settled).
    pub spent: f64,
}

impl CardLedger {
    /// Create a new ledger with the given available balance.
    pub fn new(available: f64) -> Self {
        Self {
            available,
            held: 0.0,
            spent: 0.0,
        }
    }

    /// Hold the specified amount from available into held.
    ///
    /// Moves `amount` from `available` to `held`. Returns an error
    /// if `amount` exceeds the available balance (over-hold).
    pub fn hold(&mut self, amount: f64) -> LedgerResult<()> {
        if amount > self.available {
            return Err(LedgerError::OverHold {
                amount,
                available: self.available,
            });
        }
        self.available -= amount;
        self.held += amount;
        Ok(())
    }

    /// Capture the specified amount from held into spent.
    ///
    /// Moves `amount` from `held` to `spent`. Returns an error
    /// if `amount` exceeds the held balance (over-capture).
    pub fn capture(&mut self, amount: f64) -> LedgerResult<()> {
        if amount > self.held {
            return Err(LedgerError::OverCapture {
                amount,
                held: self.held,
            });
        }
        self.held -= amount;
        self.spent += amount;
        Ok(())
    }

    /// Release the specified amount from held back to available.
    ///
    /// Moves `amount` from `held` to `available`. Returns an error
    /// if `amount` exceeds the held balance.
    pub fn release(&mut self, amount: f64) -> LedgerResult<()> {
        if amount > self.held {
            return Err(LedgerError::OverCapture {
                amount,
                held: self.held,
            });
        }
        self.held -= amount;
        self.available += amount;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_then_capture_reduces_available_and_increases_spent() {
        let mut ledger = CardLedger::new(100.0);
        assert_eq!(ledger.available, 100.0);
        assert_eq!(ledger.held, 0.0);
        assert_eq!(ledger.spent, 0.0);

        ledger.hold(30.0).unwrap();
        assert_eq!(ledger.available, 70.0);
        assert_eq!(ledger.held, 30.0);
        assert_eq!(ledger.spent, 0.0);

        ledger.capture(20.0).unwrap();
        assert_eq!(ledger.available, 70.0);
        assert_eq!(ledger.held, 10.0);
        assert_eq!(ledger.spent, 20.0);
    }

    #[test]
    fn release_restores_available() {
        let mut ledger = CardLedger::new(100.0);
        ledger.hold(30.0).unwrap();
        assert_eq!(ledger.held, 30.0);

        ledger.release(15.0).unwrap();
        assert_eq!(ledger.available, 85.0);
        assert_eq!(ledger.held, 15.0);
        assert_eq!(ledger.spent, 0.0);
    }

    #[test]
    fn capture_above_held_errors_and_leaves_balances_unchanged() {
        let mut ledger = CardLedger::new(100.0);
        ledger.hold(30.0).unwrap();

        let before = ledger.clone();
        let result = ledger.capture(50.0);
        assert!(matches!(result, Err(LedgerError::OverCapture { .. })));
        assert_eq!(ledger.available, before.available);
        assert_eq!(ledger.held, before.held);
        assert_eq!(ledger.spent, before.spent);
    }

    #[test]
    fn over_hold_returns_error() {
        let mut ledger = CardLedger::new(10.0);
        let result = ledger.hold(20.0);
        assert!(matches!(result, Err(LedgerError::OverHold { .. })));
        assert_eq!(ledger.available, 10.0);
        assert_eq!(ledger.held, 0.0);
        assert_eq!(ledger.spent, 0.0);
    }

    #[test]
    fn full_hold_capture_release_cycle() {
        let mut ledger = CardLedger::new(200.0);
        ledger.hold(100.0).unwrap();
        ledger.capture(100.0).unwrap();
        assert_eq!(ledger.available, 100.0);
        assert_eq!(ledger.held, 0.0);
        assert_eq!(ledger.spent, 100.0);

        ledger.hold(50.0).unwrap();
        ledger.release(50.0).unwrap();
        assert_eq!(ledger.available, 100.0);
        assert_eq!(ledger.held, 0.0);
        assert_eq!(ledger.spent, 100.0);
    }

    #[test]
    fn release_above_held_errors() {
        let mut ledger = CardLedger::new(100.0);
        ledger.hold(10.0).unwrap();
        let result = ledger.release(20.0);
        assert!(matches!(result, Err(LedgerError::OverCapture { .. })));
    }
}
