//! Swap prepare/submit: classic PathPaymentStrictSend only.
//!
//! Soroban/AMM/router execution is hard-disabled until a real ABI and simulation
//! assembly exist. Successful prepare always uses [`tx::ExecutionMode::ClassicPathPayment`].
//!
//! ## Sequence / concurrency limitation
//! Non-custodial prepare reserves the next account sequence for one active
//! prepare per sender. Concurrent prepares for the same G-account are rejected
//! until the prior quote expires, submits, or permanently fails. On `tx_bad_seq`,
//! clients must request a fresh prepare (Horizon sequence may have advanced).

pub mod price;
pub mod route;
pub mod store;
pub mod tx;
pub mod venue;

pub use price::*;
pub use route::*;
pub use store::*;
pub use tx::*;
pub use venue::*;
