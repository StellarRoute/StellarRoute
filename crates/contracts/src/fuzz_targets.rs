//! Fuzz targets for router entrypoint input validation (`validate_route`, `execute_swap`).
//!
//! These targets use proptest structured fuzzing (the practical approach for Soroban
//! `Env` + mocked auth). Run overnight with elevated case counts — see
//! `audit/fuzzing.md`.
//!
//! Oracle: malformed inputs must never panic; they must return a typed
//! `ContractError` (or succeed only when inputs are within valid bounds).

use crate::errors::ContractError;
use crate::router::{StellarRoute, StellarRouteClient};
use crate::test::{deploy_mock_pool, deploy_router, make_route, setup_env};
use crate::types::{Asset, PoolType, Route, RouteHop, SwapParams};
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

fn current_seq(env: &Env) -> u64 {
    env.ledger().sequence() as u64
}

fn swap_params_for(
    env: &Env,
    route: Route,
    amount_in: i128,
    min_out: i128,
    deadline: u64,
) -> SwapParams {
    SwapParams {
        route,
        amount_in,
        min_amount_out: min_out,
        recipient: Address::generate(env),
        deadline,
        not_before: 0,
        max_price_impact_bps: 0,
        max_execution_spread_bps: 0,
    }
}

fn make_route_with_meta(
    env: &Env,
    pool: &Address,
    hops: u32,
    estimated_output: i128,
    min_output: i128,
    expires_at: u64,
) -> Route {
    let mut route = make_route(env, pool, hops);
    route.estimated_output = estimated_output;
    route.min_output = min_output;
    route.expires_at = expires_at;
    route
}

/// Rebuild a multi-hop route with broken hop-0 → hop-1 asset continuity.
fn broken_continuity_route(env: &Env, pool: &Address, hops: u32) -> Route {
    let mut v = Vec::new(env);
    for i in 0..hops {
        let (source, destination) = if i == 0 {
            (Asset::Native, Asset::Native)
        } else if i == 1 {
            // Intentionally discontinuous with hop 0 destination (Native).
            (
                Asset::Soroban(Address::generate(env)),
                Asset::Native,
            )
        } else {
            (Asset::Native, Asset::Native)
        };
        v.push_back(RouteHop {
            source,
            destination,
            pool: pool.clone(),
            pool_type: PoolType::AmmConstProd,
        });
    }
    Route {
        hops: v,
        estimated_output: 0,
        min_output: 0,
        expires_at: 99_999,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Fuzz target: `validate_route` hop-count bounds.
    #[test]
    fn fuzz_validate_route_hop_bounds(hops in 0u32..12u32) {
        let env = setup_env();
        let (_, _, client) = deploy_router(&env);
        let pool = deploy_mock_pool(&env);
        client.register_pool(&pool);

        let route = make_route(&env, &pool, hops);
        let result = client.try_validate_route(&route);

        if (1..=4).contains(&hops) {
            prop_assert!(result.is_ok(), "valid hop count {} rejected: {:?}", hops, result);
        } else if hops == 0 {
            prop_assert_eq!(result, Err(Ok(ContractError::EmptyRoute)));
        } else {
            prop_assert_eq!(result, Err(Ok(ContractError::TooManyHops)));
        }
    }

    /// Fuzz target: `validate_route` amount / expiry / min-vs-estimate consistency.
    #[test]
    fn fuzz_validate_route_amounts_and_expiry(
        hops in 1u32..=4u32,
        estimated_output in -64i128..=64i128,
        min_output in -64i128..=64i128,
        expires_delta in -2i64..=4i64,
    ) {
        let env = setup_env();
        let (_, _, client) = deploy_router(&env);
        let pool = deploy_mock_pool(&env);
        client.register_pool(&pool);

        let seq = current_seq(&env);
        let expires_at = if expires_delta < 0 {
            seq.saturating_sub((-expires_delta) as u64)
        } else {
            seq.saturating_add(expires_delta as u64)
        };

        let route = make_route_with_meta(
            &env,
            &pool,
            hops,
            estimated_output,
            min_output,
            expires_at,
        );
        let result = client.try_validate_route(&route);

        if expires_at > 0 && seq > expires_at {
            prop_assert_eq!(result, Err(Ok(ContractError::RouteExpired)));
            return Ok(());
        }
        if estimated_output < 0 || min_output < 0 {
            prop_assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
            return Ok(());
        }
        if estimated_output > 0 && min_output > estimated_output {
            prop_assert_eq!(result, Err(Ok(ContractError::InvalidRoute)));
            return Ok(());
        }
        prop_assert!(result.is_ok(), "unexpected validate_route failure: {:?}", result);
    }

    /// Fuzz target: `validate_route` rejects broken hop continuity.
    #[test]
    fn fuzz_validate_route_hop_continuity(
        hops in 2u32..=4u32,
        break_continuity in any::<bool>(),
    ) {
        let env = setup_env();
        let (_, _, client) = deploy_router(&env);
        let pool = deploy_mock_pool(&env);
        client.register_pool(&pool);

        let route = if break_continuity {
            broken_continuity_route(&env, &pool, hops)
        } else {
            make_route(&env, &pool, hops)
        };

        let result = client.try_validate_route(&route);
        if break_continuity {
            prop_assert_eq!(result, Err(Ok(ContractError::InvalidRoute)));
        } else {
            prop_assert!(result.is_ok());
        }
    }

    /// Fuzz target: `validate_route` never panics on arbitrary metadata.
    #[test]
    fn fuzz_validate_route_no_panic(
        hops in 0u32..10u32,
        estimated_output in -128i128..=128i128,
        min_output in -128i128..=128i128,
        expires_at in 0u64..200u64,
        break_continuity in any::<bool>(),
        register_pool in any::<bool>(),
    ) {
        let env = setup_env();
        let (_, _, client) = deploy_router(&env);
        let pool = deploy_mock_pool(&env);
        if register_pool {
            client.register_pool(&pool);
        }

        let mut route = if break_continuity && hops >= 2 {
            broken_continuity_route(&env, &pool, hops)
        } else {
            make_route_with_meta(&env, &pool, hops, estimated_output, min_output, expires_at)
        };
        route.estimated_output = estimated_output;
        route.min_output = min_output;
        route.expires_at = expires_at;

        // Oracle: must not panic — any Outcome is acceptable.
        let _ = client.try_validate_route(&route);
    }

    /// Fuzz target: `execute_swap` amount_in bounds.
    #[test]
    fn fuzz_execute_swap_amount_bounds(amount_in in -64i128..=64i128) {
        let env = setup_env();
        let (_, _, client) = deploy_router(&env);
        let pool = deploy_mock_pool(&env);
        client.register_pool(&pool);

        let route = make_route(&env, &pool, 1);
        let params = swap_params_for(&env, route, amount_in, 0, current_seq(&env) + 100);
        let result = client.try_execute_swap(&Address::generate(&env), &params);

        if amount_in <= 0 {
            prop_assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
        } else {
            prop_assert!(result.is_ok(), "positive amount rejected: {:?}", result);
        }
    }

    /// Fuzz target: `execute_swap` min_amount_out / route.min_output / bps guards.
    #[test]
    fn fuzz_execute_swap_guards(
        min_amount_out in -32i128..=32i128,
        route_min_output in -32i128..=32i128,
        impact_bps in 0u32..20_000u32,
        spread_bps in 0u32..20_000u32,
    ) {
        let env = setup_env();
        let (_, _, client) = deploy_router(&env);
        let pool = deploy_mock_pool(&env);
        client.register_pool(&pool);

        let mut route = make_route(&env, &pool, 1);
        route.min_output = route_min_output;
        let params = SwapParams {
            route,
            amount_in: 1_000,
            min_amount_out,
            recipient: Address::generate(&env),
            deadline: current_seq(&env) + 100,
            not_before: 0,
            max_price_impact_bps: impact_bps,
            max_execution_spread_bps: spread_bps,
        };

        let result = client.try_execute_swap(&Address::generate(&env), &params);

        if min_amount_out < 0 || route_min_output < 0 {
            prop_assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
            return Ok(());
        }
        if impact_bps > 10_000 || spread_bps > 10_000 {
            prop_assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
            return Ok(());
        }
        // Valid guards: either succeeds or fails later on slippage/impact — never panics.
        let _ = result;
    }

    /// Fuzz target: `execute_swap` hop count (empty / too many → InvalidRoute).
    #[test]
    fn fuzz_execute_swap_hop_bounds(hops in 0u32..10u32) {
        let env = setup_env();
        let (_, _, client) = deploy_router(&env);
        let pool = deploy_mock_pool(&env);
        client.register_pool(&pool);

        let route = make_route(&env, &pool, hops);
        let params = swap_params_for(&env, route, 1_000, 0, current_seq(&env) + 100);
        let result = client.try_execute_swap(&Address::generate(&env), &params);

        if hops == 0 || hops > 4 {
            prop_assert_eq!(result, Err(Ok(ContractError::InvalidRoute)));
        } else {
            prop_assert!(result.is_ok(), "valid hops {} rejected: {:?}", hops, result);
        }
    }

    /// Fuzz target: `execute_swap` deadline / not_before window.
    #[test]
    fn fuzz_execute_swap_time_window(
        deadline_offset in -5i64..=5i64,
        not_before_offset in -5i64..=5i64,
    ) {
        let env = setup_env();
        let (_, _, client) = deploy_router(&env);
        let pool = deploy_mock_pool(&env);
        client.register_pool(&pool);

        let seq = current_seq(&env) as i64;
        let deadline = (seq + deadline_offset).max(0) as u64;
        let not_before = (seq + not_before_offset).max(0) as u64;

        let route = make_route(&env, &pool, 1);
        let params = SwapParams {
            route,
            amount_in: 1_000,
            min_amount_out: 0,
            recipient: Address::generate(&env),
            deadline,
            not_before,
            max_price_impact_bps: 0,
            max_execution_spread_bps: 0,
        };

        let result = client.try_execute_swap(&Address::generate(&env), &params);
        let now = current_seq(&env);

        if now > deadline {
            prop_assert_eq!(result, Err(Ok(ContractError::DeadlineExceeded)));
            return Ok(());
        }
        if now < not_before {
            prop_assert_eq!(result, Err(Ok(ContractError::ExecutionTooEarly)));
            return Ok(());
        }
        prop_assert!(result.is_ok(), "in-window swap failed: {:?}", result);
    }

    /// Fuzz target: `execute_swap` rejects the router contract as recipient.
    #[test]
    fn fuzz_execute_swap_invalid_recipient(use_router_as_recipient in any::<bool>()) {
        let env = setup_env();
        let (_, _, client) = deploy_router(&env);
        let pool = deploy_mock_pool(&env);
        client.register_pool(&pool);

        let route = make_route(&env, &pool, 1);
        let recipient = if use_router_as_recipient {
            client.address.clone()
        } else {
            Address::generate(&env)
        };
        let params = SwapParams {
            route,
            amount_in: 1_000,
            min_amount_out: 0,
            recipient,
            deadline: current_seq(&env) + 100,
            not_before: 0,
            max_price_impact_bps: 0,
            max_execution_spread_bps: 0,
        };

        let result = client.try_execute_swap(&Address::generate(&env), &params);
        if use_router_as_recipient {
            prop_assert_eq!(result, Err(Ok(ContractError::InvalidRecipient)));
        } else {
            prop_assert!(result.is_ok());
        }
    }

    /// Fuzz target: `execute_swap` never panics on malformed inputs.
    #[test]
    fn fuzz_execute_swap_no_panic(
        amount_in in -256i128..=256i128,
        min_amount_out in -256i128..=256i128,
        hops in 0u32..8u32,
        impact_bps in 0u32..30_000u32,
        spread_bps in 0u32..30_000u32,
        deadline_offset in -20i64..=20i64,
        not_before_offset in -20i64..=20i64,
        register_pool in any::<bool>(),
    ) {
        let env = setup_env();
        let (_, _, client) = deploy_router(&env);
        let pool = deploy_mock_pool(&env);
        if register_pool {
            client.register_pool(&pool);
        }

        let seq = current_seq(&env) as i64;
        let mut route = make_route(&env, &pool, hops);
        route.min_output = min_amount_out;

        let params = SwapParams {
            route,
            amount_in,
            min_amount_out,
            recipient: Address::generate(&env),
            deadline: (seq + deadline_offset).max(0) as u64,
            not_before: (seq + not_before_offset).max(0) as u64,
            max_price_impact_bps: impact_bps,
            max_execution_spread_bps: spread_bps,
        };

        // Oracle: must not panic.
        let _ = client.try_execute_swap(&Address::generate(&env), &params);
    }

    /// Fuzz target: output bounded by input and slippage min_out enforced.
    #[test]
    fn fuzz_execute_swap_output_and_slippage(
        amount_in in 10i128..=10_000_000_i128,
        fee_rate in 0u32..=1000u32,
        slippage_factor in 0i128..20i128,
    ) {
        let env = setup_env();
        let admin = Address::generate(&env);
        let fee_to = Address::generate(&env);
        let id = env.register_contract(None, StellarRoute);
        let client = StellarRouteClient::new(&env, &id);
        client.initialize(&admin, &fee_rate, &fee_to, &None, &None, &None, &None, &None);

        let pool = deploy_mock_pool(&env);
        client.register_pool(&pool);

        let route = make_route(&env, &pool, 1);

        // MockAmmPool returns 99%; router applies fee_rate bps on that output.
        let pool_out = amount_in * 99 / 100;
        let fee = pool_out * (fee_rate as i128) / 10000;
        let expected_output = pool_out - fee;

        let min_out_ok = (expected_output - (expected_output * slippage_factor / 100)).max(0);
        let params_ok = swap_params_for(
            &env,
            route.clone(),
            amount_in,
            min_out_ok,
            current_seq(&env) + 100,
        );
        let result = client.try_execute_swap(&Address::generate(&env), &params_ok);
        prop_assert!(
            result.is_ok(),
            "Expected swap to succeed with min_out = {}, got {:?}",
            min_out_ok,
            result
        );
        let swap_res = result.unwrap();
        prop_assert!(
            swap_res.amount_out <= amount_in,
            "Output {} cannot exceed input {}",
            swap_res.amount_out,
            amount_in
        );
        prop_assert_eq!(swap_res.amount_out, expected_output);

        let min_out_fail = expected_output + 1;
        let params_fail = swap_params_for(
            &env,
            route,
            amount_in,
            min_out_fail,
            current_seq(&env) + 100,
        );
        let result_fail = client.try_execute_swap(&Address::generate(&env), &params_fail);
        prop_assert_eq!(
            result_fail,
            Err(Ok(ContractError::SlippageExceeded)),
            "Expected SlippageExceeded when min_out = {}",
            min_out_fail
        );
    }
}

#[cfg(test)]
mod smoke {
    use super::*;

    /// Deterministic smoke that the fuzz module helpers compile and run.
    #[test]
    fn fuzz_helpers_build_malformed_route() {
        let env = setup_env();
        let pool = Address::generate(&env);
        let mut hops = Vec::new(&env);
        hops.push_back(RouteHop {
            source: Asset::Native,
            destination: Asset::Native,
            pool: pool.clone(),
            pool_type: PoolType::AmmConstProd,
        });
        let route = Route {
            hops,
            estimated_output: -1,
            min_output: -1,
            expires_at: 0,
        };
        assert_eq!(route.estimated_output, -1);
    }
}
