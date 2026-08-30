//! Immutable Circle CCTP source pins for attestation parity.

/// `circlefin/evm-cctp-contracts` commit pinning `Attestable.sol`.
pub const EVM_CCTP_CONTRACTS_COMMIT: &str = "a92a2b4e7e6e";
pub const EVM_ATTESTABLE_PATH: &str = "src/roles/Attestable.sol";

/// `circlefin/stellar-cctp` commit pinning Soroban attestable storage + fixtures.
pub const STELLAR_CCTP_COMMIT: &str = "45746f2c8031";
pub const STELLAR_ATTESTABLE_STORAGE_PATH: &str = "packages/cctp-roles/src/attestable/storage.rs";
pub const STELLAR_ATTESTABLE_FIXTURES_PATH: &str =
    "packages/cctp-roles/src/test_utils/attestable.rs";
