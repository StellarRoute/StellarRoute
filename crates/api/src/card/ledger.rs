#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardLedger {
    pub available: i64,
    pub held: i64,
    pub spent: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    InvalidAmount,
    InsufficientAvailable,
    InsufficientHeld,
}

impl CardLedger {
    pub fn new(available: i64) -> Result<Self, LedgerError> {
        if available < 0 {
            return Err(LedgerError::InvalidAmount);
        }

        Ok(Self {
            available,
            held: 0,
            spent: 0,
        })
    }

    pub fn hold(&mut self, amount: i64) -> Result<(), LedgerError> {
        if amount < 0 {
            return Err(LedgerError::InvalidAmount);
        }
        if amount > self.available {
            return Err(LedgerError::InsufficientAvailable);
        }

        self.available -= amount;
        self.held += amount;
        Ok(())
    }

    pub fn capture(&mut self, amount: i64) -> Result<(), LedgerError> {
        if amount < 0 {
            return Err(LedgerError::InvalidAmount);
        }
        if amount > self.held {
            return Err(LedgerError::InsufficientHeld);
        }

        self.held -= amount;
        self.spent += amount;
        Ok(())
    }

    pub fn release(&mut self, amount: i64) -> Result<(), LedgerError> {
        if amount < 0 {
            return Err(LedgerError::InvalidAmount);
        }
        if amount > self.held {
            return Err(LedgerError::InsufficientHeld);
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
    fn hold_then_capture_moves_amount_to_spent() {
        let mut ledger = CardLedger::new(1_000).unwrap();

        ledger.hold(250).unwrap();
        ledger.capture(250).unwrap();

        assert_eq!(
            ledger,
            CardLedger {
                available: 750,
                held: 0,
                spent: 250,
            }
        );
    }

    #[test]
    fn release_restores_available_balance() {
        let mut ledger = CardLedger::new(1_000).unwrap();

        ledger.hold(250).unwrap();
        ledger.release(250).unwrap();

        assert_eq!(
            ledger,
            CardLedger {
                available: 1_000,
                held: 0,
                spent: 0,
            }
        );
    }

    #[test]
    fn over_capture_leaves_balances_unchanged() {
        let mut ledger = CardLedger::new(1_000).unwrap();
        ledger.hold(250).unwrap();
        let before = ledger;

        assert_eq!(ledger.capture(251), Err(LedgerError::InsufficientHeld));
        assert_eq!(ledger, before);
    }

    #[test]
    fn over_hold_returns_error() {
        let mut ledger = CardLedger::new(1_000).unwrap();
        assert_eq!(ledger.hold(1_500), Err(LedgerError::InsufficientAvailable));
        assert_eq!(ledger.available, 1_000);
        assert_eq!(ledger.held, 0);
        assert_eq!(ledger.spent, 0);
    }

    #[test]
    fn full_hold_capture_release_cycle() {
        let mut ledger = CardLedger::new(1_000).unwrap();
        ledger.hold(400).unwrap();
        ledger.capture(250).unwrap();
        ledger.release(150).unwrap();
        assert_eq!(
            ledger,
            CardLedger {
                available: 750,
                held: 0,
                spent: 250,
            }
        );
    }

    #[test]
    fn release_above_held_errors() {
        let mut ledger = CardLedger::new(1_000).unwrap();
        ledger.hold(100).unwrap();
        let before = ledger;
        assert_eq!(ledger.release(101), Err(LedgerError::InsufficientHeld));
        assert_eq!(ledger, before);
    }
}
