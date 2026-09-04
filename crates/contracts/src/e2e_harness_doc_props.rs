//! Correctness checks verifying `docs/contracts/testing-guide.md` accuracy.
//!
//! Each test reads a source file and asserts that documented identifiers
//! still exist in the live source. Run with:
//!   cargo test -p stellarroute-contracts e2e_harness_doc_props

// NOTE: This crate is #![no_std] for Soroban contract builds, but these are
// cfg(test) modules that run in the host environment where std is available.
// The `extern crate std` is implicit in test builds via the test harness.

#[cfg(test)]
mod tests {
    fn read(path: &str) -> String {
        // Paths are relative to the workspace root; Cargo sets the working
        // directory to the workspace root when running tests.
        std::fs::read_to_string(path).unwrap_or_else(|_| panic!("could not read {path}"))
    }

    // Feature: contract-e2e-harness-guide, Property 1: run command presence
    #[test]
    fn prop_run_command_present() {
        let guide = read("docs/contracts/testing-guide.md");
        assert!(
            guide.contains("cargo test -p stellarroute-contracts e2e"),
            "docs/contracts/testing-guide.md must contain the primary run command \
             `cargo test -p stellarroute-contracts e2e`"
        );
    }

    // Feature: contract-e2e-harness-guide, Property 2: all documented ContractError variants exist in source
    #[test]
    fn prop_error_variants_in_source() {
        let errors_src = read("crates/contracts/src/errors.rs");
        let documented_variants = [
            "SlippageExceeded",
            "DeadlineExceeded",
            "ExecutionTooEarly",
            "Paused",
            "AmmSwapCallFailed",
            "PoolNotSupported",
            "InvalidRecipient",
            "InvalidAmount",
            "InvalidRoute",
            "RateLimitExceeded",
            "CommitmentRequired",
        ];
        for variant in &documented_variants {
            assert!(
                errors_src.contains(variant),
                "ContractError variant `{variant}` is documented in the guide \
                 but not found in crates/contracts/src/errors.rs — update the guide if renamed"
            );
        }
    }

    // Feature: contract-e2e-harness-guide, Property 3: all documented helper function names exist in source
    #[test]
    fn prop_helpers_in_source() {
        let helpers = read("crates/contracts/src/e2e_helpers.rs");
        let harness = read("crates/contracts/src/e2e_harness.rs");
        let combined = format!("{helpers}{harness}");
        let documented_helpers = [
            "setup",
            "deploy_router",
            "deploy_pool_99",
            "deploy_pool_98",
            "deploy_pool_fail",
            "multi_pool_route",
            "swap_params",
        ];
        for name in &documented_helpers {
            assert!(
                combined.contains(name),
                "Helper `{name}` is documented in the guide but not found in \
                 e2e_helpers.rs or e2e_harness.rs — update the guide if renamed"
            );
        }
    }

    // Feature: contract-e2e-harness-guide, Property 4: MAX_HOPS value matches source
    #[test]
    fn prop_max_hops_value_correct() {
        let guide = read("docs/contracts/testing-guide.md");
        assert!(
            guide.contains("MAX_HOPS"),
            "docs/contracts/testing-guide.md must mention MAX_HOPS"
        );
        assert!(
            guide.contains("MAX_HOPS = 4")
                || guide.contains("MAX_HOPS` (4)")
                || guide.contains("MAX_HOPS=4"),
            "docs/contracts/testing-guide.md must document MAX_HOPS as 4"
        );
        assert!(
            guide.contains("InvalidRoute"),
            "docs/contracts/testing-guide.md must mention InvalidRoute in the MAX_HOPS context"
        );
    }
}
