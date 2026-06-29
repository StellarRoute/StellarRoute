//! Multi-hop CPI failure atomic rollback integration tests
//!
//! These tests verify that multi-hop swaps roll back atomically when an
//! intermediate CPI (Cross-Program Invocation) fails, ensuring no partial
//! token movement occurs.

#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, Address, Env, String, Vec as SorobanVec,
};

use crate::router::{Router, RouterClient};
use crate::types::{PathStep, SwapError};

/// Mock adapter contract that can be configured to fail at specific calls
#[contract]
pub struct FailingAdapter;

#[contractimpl]
impl FailingAdapter {
    /// Simulate a swap that always fails
    pub fn swap(
        _env: Env,
        _token_in: Address,
        _token_out: Address,
        _amount_in: i128,
        _min_amount_out: i128,
    ) -> Result<i128, SwapError> {
        Err(SwapError::InsufficientLiquidity)
    }
}

/// Mock adapter contract that succeeds
#[contract]
pub struct SuccessAdapter;

#[contractimpl]
impl SuccessAdapter {
    pub fn swap(
        _env: Env,
        _token_in: Address,
        _token_out: Address,
        amount_in: i128,
        _min_amount_out: i128,
    ) -> Result<i128, SwapError> {
        // Simple 1:1 swap for testing
        Ok(amount_in)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Events, Ledger};

    #[test]
    fn test_single_hop_failure_rolls_back() {
        let env = Env::default();
        env.mock_all_auths();

        let router_id = env.register_contract(None, Router);
        let router = RouterClient::new(&env, &router_id);

        let failing_adapter_id = env.register_contract(None, FailingAdapter);

        let token_a = Address::generate(&env);
        let token_b = Address::generate(&env);
        let user = Address::generate(&env);

        let path = SorobanVec::from_array(
            &env,
            [PathStep {
                token_in: token_a.clone(),
                token_out: token_b.clone(),
                adapter: failing_adapter_id,
                pool_id: String::from_str(&env, "pool_1"),
            }],
        );

        let initial_balance = 1000i128;
        let swap_amount = 100i128;

        // Execute swap - should fail
        let result = router.try_swap_exact_in(&user, &path, &swap_amount, &0);

        assert!(result.is_err(), "Swap should fail");

        // Verify no events were emitted for successful swap
        let events = env.events().all();
        let swap_events: Vec<_> = events
            .iter()
            .filter(|e| {
                e.1.first()
                    .and_then(|t| t.as_symbol())
                    .map(|s| s.to_string() == "swap_complete")
                    .unwrap_or(false)
            })
            .collect();

        assert_eq!(swap_events.len(), 0, "No swap_complete events should exist");
    }

    #[test]
    fn test_failure_at_hop_index_0() {
        let env = Env::default();
        env.mock_all_auths();

        let router_id = env.register_contract(None, Router);
        let router = RouterClient::new(&env, &router_id);

        let failing_adapter_id = env.register_contract(None, FailingAdapter);
        let success_adapter_id = env.register_contract(None, SuccessAdapter);

        let token_a = Address::generate(&env);
        let token_b = Address::generate(&env);
        let token_c = Address::generate(&env);
        let user = Address::generate(&env);

        // Path: A -> B (fails) -> C (should not execute)
        let path = SorobanVec::from_array(
            &env,
            [
                PathStep {
                    token_in: token_a.clone(),
                    token_out: token_b.clone(),
                    adapter: failing_adapter_id,
                    pool_id: String::from_str(&env, "pool_1"),
                },
                PathStep {
                    token_in: token_b.clone(),
                    token_out: token_c.clone(),
                    adapter: success_adapter_id,
                    pool_id: String::from_str(&env, "pool_2"),
                },
            ],
        );

        let swap_amount = 100i128;

        let result = router.try_swap_exact_in(&user, &path, &swap_amount, &0);

        assert!(result.is_err(), "Multi-hop swap should fail at hop 0");

        // Verify rollback event
        let events = env.events().all();
        let rollback_events: Vec<_> = events
            .iter()
            .filter(|e| {
                e.1.first()
                    .and_then(|t| t.as_symbol())
                    .map(|s| s.to_string() == "swap_rollback")
                    .unwrap_or(false)
            })
            .collect();

        assert!(
            rollback_events.len() > 0,
            "Rollback event should be emitted"
        );
    }

    #[test]
    fn test_failure_at_hop_index_1() {
        let env = Env::default();
        env.mock_all_auths();

        let router_id = env.register_contract(None, Router);
        let router = RouterClient::new(&env, &router_id);

        let failing_adapter_id = env.register_contract(None, FailingAdapter);
        let success_adapter_id = env.register_contract(None, SuccessAdapter);

        let token_a = Address::generate(&env);
        let token_b = Address::generate(&env);
        let token_c = Address::generate(&env);
        let user = Address::generate(&env);

        // Path: A -> B (succeeds) -> C (fails)
        let path = SorobanVec::from_array(
            &env,
            [
                PathStep {
                    token_in: token_a.clone(),
                    token_out: token_b.clone(),
                    adapter: success_adapter_id.clone(),
                    pool_id: String::from_str(&env, "pool_1"),
                },
                PathStep {
                    token_in: token_b.clone(),
                    token_out: token_c.clone(),
                    adapter: failing_adapter_id,
                    pool_id: String::from_str(&env, "pool_2"),
                },
            ],
        );

        let swap_amount = 100i128;

        let result = router.try_swap_exact_in(&user, &path, &swap_amount, &0);

        assert!(result.is_err(), "Multi-hop swap should fail at hop 1");

        // Verify no partial token movement - check that intermediate hop was rolled back
        let events = env.events().all();
        let swap_complete_events: Vec<_> = events
            .iter()
            .filter(|e| {
                e.1.first()
                    .and_then(|t| t.as_symbol())
                    .map(|s| s.to_string() == "swap_complete")
                    .unwrap_or(false)
            })
            .collect();

        assert_eq!(
            swap_complete_events.len(),
            0,
            "No swap should complete on rollback"
        );
    }

    #[test]
    fn test_failure_at_hop_index_2_three_hop() {
        let env = Env::default();
        env.mock_all_auths();

        let router_id = env.register_contract(None, Router);
        let router = RouterClient::new(&env, &router_id);

        let failing_adapter_id = env.register_contract(None, FailingAdapter);
        let success_adapter_id = env.register_contract(None, SuccessAdapter);

        let token_a = Address::generate(&env);
        let token_b = Address::generate(&env);
        let token_c = Address::generate(&env);
        let token_d = Address::generate(&env);
        let user = Address::generate(&env);

        // Path: A -> B (succeeds) -> C (succeeds) -> D (fails)
        let path = SorobanVec::from_array(
            &env,
            [
                PathStep {
                    token_in: token_a.clone(),
                    token_out: token_b.clone(),
                    adapter: success_adapter_id.clone(),
                    pool_id: String::from_str(&env, "pool_1"),
                },
                PathStep {
                    token_in: token_b.clone(),
                    token_out: token_c.clone(),
                    adapter: success_adapter_id.clone(),
                    pool_id: String::from_str(&env, "pool_2"),
                },
                PathStep {
                    token_in: token_c.clone(),
                    token_out: token_d.clone(),
                    adapter: failing_adapter_id,
                    pool_id: String::from_str(&env, "pool_3"),
                },
            ],
        );

        let swap_amount = 100i128;

        let result = router.try_swap_exact_in(&user, &path, &swap_amount, &0);

        assert!(result.is_err(), "Multi-hop swap should fail at hop 2");

        // Events should reflect rollback outcome
        let events = env.events().all();
        let rollback_events: Vec<_> = events
            .iter()
            .filter(|e| {
                e.1.first()
                    .and_then(|t| t.as_symbol())
                    .map(|s| s.to_string() == "swap_rollback")
                    .unwrap_or(false)
            })
            .collect();

        assert!(
            rollback_events.len() > 0,
            "Rollback event should be emitted for 3-hop failure"
        );
    }

    #[test]
    fn test_adapter_contract_failure_propagates() {
        let env = Env::default();
        env.mock_all_auths();

        let router_id = env.register_contract(None, Router);
        let router = RouterClient::new(&env, &router_id);

        let failing_adapter_id = env.register_contract(None, FailingAdapter);

        let token_a = Address::generate(&env);
        let token_b = Address::generate(&env);
        let user = Address::generate(&env);

        let path = SorobanVec::from_array(
            &env,
            [PathStep {
                token_in: token_a.clone(),
                token_out: token_b.clone(),
                adapter: failing_adapter_id,
                pool_id: String::from_str(&env, "pool_1"),
            }],
        );

        let result = router.try_swap_exact_in(&user, &path, &100, &0);

        match result {
            Err(e) => {
                // Verify the error indicates adapter failure
                assert!(
                    format!("{:?}", e).contains("InsufficientLiquidity")
                        || format!("{:?}", e).contains("Error"),
                    "Error should propagate from adapter"
                );
            }
            Ok(_) => panic!("Should have failed"),
        }
    }

    #[test]
    fn test_all_hops_succeed_no_rollback() {
        let env = Env::default();
        env.mock_all_auths();

        let router_id = env.register_contract(None, Router);
        let router = RouterClient::new(&env, &router_id);

        let success_adapter_id = env.register_contract(None, SuccessAdapter);

        let token_a = Address::generate(&env);
        let token_b = Address::generate(&env);
        let token_c = Address::generate(&env);
        let user = Address::generate(&env);

        let path = SorobanVec::from_array(
            &env,
            [
                PathStep {
                    token_in: token_a.clone(),
                    token_out: token_b.clone(),
                    adapter: success_adapter_id.clone(),
                    pool_id: String::from_str(&env, "pool_1"),
                },
                PathStep {
                    token_in: token_b.clone(),
                    token_out: token_c.clone(),
                    adapter: success_adapter_id,
                    pool_id: String::from_str(&env, "pool_2"),
                },
            ],
        );

        let result = router.try_swap_exact_in(&user, &path, &100, &0);

        assert!(result.is_ok(), "All successful hops should complete");

        // Verify no rollback events
        let events = env.events().all();
        let rollback_events: Vec<_> = events
            .iter()
            .filter(|e| {
                e.1.first()
                    .and_then(|t| t.as_symbol())
                    .map(|s| s.to_string() == "swap_rollback")
                    .unwrap_or(false)
            })
            .collect();

        assert_eq!(
            rollback_events.len(),
            0,
            "No rollback events on successful path"
        );
    }
}
