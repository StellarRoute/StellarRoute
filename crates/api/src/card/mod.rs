//! Card module for StellarRoute API.
//!
//! Contains card ledger logic for tracking hold, capture, and release operations,
//! and card store infrastructure for authorizations and USDC holds.
//! Every route in this module is gated by `CARD_ENABLED`. When the flag is
//! unset or false the routes answer `404`.

pub mod ledger;
pub mod store;

pub use ledger::{CardLedger, LedgerError, LedgerResult};
pub use store::{CardStore, InMemoryCardStore};
