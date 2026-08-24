//! Pinned Circle Stellar CCTP v2 event layout references for adversarial tests.
//!
//! Source: `circlefin/stellar-cctp` @ commit `45746f2c8031`
//! - `contracts/token-messenger-minter-v2/src/lib.rs` (`deposit_for_burn`)
//! - `contracts/message-transmitter-v2/src/lib.rs` (`message_sent`)

pub const STELLAR_CCTP_COMMIT: &str = "45746f2c8031";

pub const FIXTURE_CANONICAL_BURN_AMOUNT: i128 = 1_000_000;
pub const FIXTURE_DESTINATION_DOMAIN: u32 = 0;
pub const FIXTURE_MIN_FINALITY: u32 = 2000;
