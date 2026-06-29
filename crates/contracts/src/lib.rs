#![no_std]
// The `initialize` entrypoint takes 9 args because bootstrapping multi-sig,
// upgrade tracking, and core config in one atomic call is intentional.
// The Soroban #[contractimpl] macro also generates client wrappers that
// inherit this signature, so the allow must be crate-wide.
#![allow(clippy::too_many_arguments)]

pub mod adapters;
pub mod constant_product_adapter;
pub mod errors;
pub mod events;
pub mod governance;
pub mod router;
pub mod storage;
pub mod tokens;
pub mod types;
pub mod upgrade;

#[cfg(test)]
mod benchmarks;
#[cfg(test)]
mod e2e_harness;
#[cfg(test)]
mod test;
