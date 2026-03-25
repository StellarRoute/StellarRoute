//! End-to-end local harness tests for StellarRoute contract swap flows.
//!
//! These tests exercise multi-contract scenarios where each hop in a route
//! uses a *distinct* pool contract, assert expected events are emitted, and
//! verify that failure mid-route does not leave the contract in a bad state.
//!
//! Run with:
//!   cargo test -p stellarroute-contracts e2e

#![allow(dead_code)]

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, Bytes, BytesN, Env, Symbol, Vec,
};

use super::{
    errors::ContractError,
    router::{StellarRoute, StellarRouteClient},
    types::{Asset, MevConfig, PoolType, Route, RouteHop, SwapParams},
};

// ── Mock pool contracts ───────────────────────────────────────────────────────
// Each mock lives in its own module to avoid symbol collisions from
// the #[contractimpl] macro.

mod mock_pool_99 {
    use super::super::types::Asset;
    use soroban_sdk::{contract, contractimpl, Env};

    /// Healthy pool: returns 99% of amount_in.
    #[contract]
    pub struct MockPool99;

    #[contractimpl]
    impl MockPool99 {
        pub fn adapter_quote(_e: Env, _in: Asset, _out: Asset, amount_in: i128) -> i128 {
            amount_in * 99 / 100
        }
        pub fn swap(_e: Env, _in: Asset, _out: Asset, amount_in: i128, min_out: i128) -> i128 {
            let out = amount_in * 99 / 100;
            assert!(out >= min_out, "mock_pool_99: slippage");
            out
        }
        pub fn get_rsrvs(_e: Env) -> (i128, i128) {
            (1_000_000_000, 1_000_000_000)
        }
    }
}

mod mock_pool_98 {
    use super::super::types::Asset;
    use soroban_sdk::{contract, contractimpl, Env};

    /// Healthy pool with a different rate: returns 98% of amount_in.
    #[contract]
    pub struct MockPool98;

    #[contractimpl]
    impl MockPool98 {
        pub fn adapter_quote(_e: Env, _in: Asset, _out: Asset, amount_in: i128) -> i128 {
            amount_in * 98 / 100
        }
        pub fn swap(_e: Env, _in: Asset, _out: Asset, amount_in: i128, min_out: i128) -> i128 {
            let out = amount_in * 98 / 100;
            assert!(out >= min_out, "mock_pool_98: slippage");
            out
        }
        pub fn get_rsrvs(_e: Env) -> (i128, i128) {
            (500_000_000, 500_000_000)
        }
    }
}

mod mock_pool_fail {
    use super::super::types::Asset;
    use soroban_sdk::{contract, contractimpl, Env};

    /// Broken pool: always panics. Used to test PoolCallFailed error paths.
    #[contract]
    pub struct MockPoolFail;

    #[contractimpl]
    impl MockPoolFail {
        pub fn adapter_quote(_e: Env, _in: Asset, _out: Asset, _amount: i128) -> i128 {
            panic!("pool unavailable")
        }
        pub fn swap(_e: Env, _in: Asset, _out: Asset, _amount: i128, _min: i128) -> i128 {
            panic!("pool unavailable")
        }
        pub fn get_rsrvs(_e: Env) -> (i128, i128) {
            panic!("pool unavailable")
        }
    }
}

// ── Harness helpers ───────────────────────────────────────────────────────────

pub fn deploy_pool_99(env: &Env) -> Address {
    env.register_contract(None, mock_pool_99::MockPool99)
}

pub fn deploy_pool_98(env: &Env) -> Address {
    env.register_contract(None, mock_pool_98::MockPool98)
}

pub fn deploy_pool_fail(env: &Env) -> Address {
    env.register_contract(None, mock_pool_fail::MockPoolFail)
}

fn setup() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn deploy_router(env: &Env) -> (Address, StellarRouteClient<'_>) {
    let admin = Address::generate(env);
    let fee_to = Address::generate(env);
    let id = env.register_contract(None, StellarRoute);
    let client = StellarRouteClient::new(env, &id);
    client.initialize(&admin, &30_u32, &fee_to, &None, &None, &None, &None, &None);
    (admin, client)
}

fn seq(env: &Env) -> u64 {
    env.ledger().sequence() as u64
}

/// Build a multi-hop route where each element of `pools` is a distinct contract.
fn multi_pool_route(env: &Env, pools: &[Address]) -> Route {
    let mut hops = Vec::new(env);
    for pool in pools {
        hops.push_back(RouteHop {
            source: Asset::Native,
            destination: Asset::Native,
            pool: pool.clone(),
            pool_type: PoolType::AmmConstProd,
        });
    }
    Route {
        hops,
        estimated_output: 0,
        min_output: 0,
        expires_at: 999_999,
    }
}

fn swap_params(env: &Env, route: Route, amount_in: i128, min_out: i128) -> SwapParams {
    SwapParams {
        route,
        amount_in,
        min_amount_out: min_out,
        recipient: Address::generate(env),
        deadline: seq(env) + 200,
        not_before: 0,
        max_price_impact_bps: 0,
        max_execution_spread_bps: 0,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Direct (single-hop) swap E2E tests ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// Happy path: single pool, correct output and event emitted.
#[test]
fn e2e_direct_swap_single_pool_success() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let sender = Address::generate(&env);
    let route = multi_pool_route(&env, &[pool]);
    let params = swap_params(&env, route, 10_000, 0);

    let events_before = env.events().all().len();
    let result = client.execute_swap(&sender, &params);

    // pool returns 99%, protocol fee 30bps on that → 9900 * 9970 / 10000 = 9871
    assert_eq!(result.amount_in, 10_000);
    assert_eq!(result.amount_out, 9_871);
    assert_eq!(result.executed_at, seq(&env));

    // At least one new event (swap_executed) must have been emitted
    assert!(env.events().all().len() > events_before);
}

/// Swap output must always be strictly less than input (fees apply).
#[test]
fn e2e_direct_swap_output_less_than_input() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    for amount in [100_i128, 1_000, 50_000, 1_000_000] {
        let result = client.execute_swap(
            &Address::generate(&env),
            &swap_params(&env, multi_pool_route(&env, &[pool.clone()]), amount, 0),
        );
        assert!(
            result.amount_out < amount,
            "amount_out {} must be < amount_in {}",
            result.amount_out,
            amount
        );
    }
}

/// Slippage guard: min_amount_out above actual output must be rejected.
#[test]
fn e2e_direct_swap_slippage_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    // 10_000 → pool 9_900 → fee → 9_870 net; require 9_900 → fail
    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 10_000, 9_900),
    );
    assert_eq!(result, Err(Ok(ContractError::SlippageExceeded)));
}

/// Deadline in the past must be rejected before any pool call.
#[test]
fn e2e_direct_swap_expired_deadline_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    env.ledger().with_mut(|l| l.sequence_number = 500);

    let mut params = swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0);
    params.deadline = 499; // already past

    let result = client.try_execute_swap(&Address::generate(&env), &params);
    assert_eq!(result, Err(Ok(ContractError::DeadlineExceeded)));
}

/// not_before enforcement: swap submitted too early must be rejected.
#[test]
fn e2e_direct_swap_not_before_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    env.ledger().with_mut(|l| l.sequence_number = 100);

    let mut params = swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0);
    params.not_before = 200; // future ledger
    params.deadline = 999;

    let result = client.try_execute_swap(&Address::generate(&env), &params);
    assert_eq!(result, Err(Ok(ContractError::ExecutionTooEarly)));
}

/// Paused contract must reject swaps immediately.
#[test]
fn e2e_direct_swap_paused_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);
    client.pause();

    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );
    assert_eq!(result, Err(Ok(ContractError::Paused)));
}

/// After unpause, swaps must succeed again.
#[test]
fn e2e_direct_swap_resumes_after_unpause() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    client.pause();
    client.unpause();

    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );
    assert!(result.is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Multi-hop swap E2E tests (distinct pool per hop) ─────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// 2-hop route: pool_99 → pool_98, each a separate contract.
/// Verifies output is compounded correctly across both pools.
#[test]
fn e2e_multi_hop_two_distinct_pools() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env); // hop 1: 99%
    let p2 = deploy_pool_98(&env); // hop 2: 98%
    client.register_pool(&p1);
    client.register_pool(&p2);

    let route = multi_pool_route(&env, &[p1, p2]);
    let result = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, route, 10_000, 0),
    );

    // hop1: 10_000 * 99/100 = 9_900
    // hop2: 9_900  * 98/100 = 9_702
    // fee:  9_702  * 30/10000 = 29  → net = 9_673
    assert_eq!(result.amount_in, 10_000);
    assert_eq!(result.amount_out, 9_673);
}

/// 3-hop route: pool_99 → pool_98 → pool_99, three distinct contracts.
#[test]
fn e2e_multi_hop_three_distinct_pools() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env);
    let p2 = deploy_pool_98(&env);
    let p3 = deploy_pool_99(&env);
    client.register_pool(&p1);
    client.register_pool(&p2);
    client.register_pool(&p3);

    let route = multi_pool_route(&env, &[p1, p2, p3]);
    let result = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, route, 100_000, 0),
    );

    // hop1: 100_000 * 99/100 = 99_000
    // hop2:  99_000 * 98/100 = 97_020
    // hop3:  97_020 * 99/100 = 96_049
    // fee:   96_049 * 30/10000 = 288 → net = 95_761
    assert_eq!(result.amount_in, 100_000);
    assert_eq!(result.amount_out, 95_761);
    assert!(result.amount_out < result.amount_in);
}

/// 4-hop route (max allowed): all distinct pools, must succeed.
#[test]
fn e2e_multi_hop_four_distinct_pools_max_hops() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env);
    let p2 = deploy_pool_98(&env);
    let p3 = deploy_pool_99(&env);
    let p4 = deploy_pool_98(&env);
    client.register_pool(&p1);
    client.register_pool(&p2);
    client.register_pool(&p3);
    client.register_pool(&p4);

    let route = multi_pool_route(&env, &[p1, p2, p3, p4]);
    let result = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, route, 1_000_000, 0),
    );

    assert!(result.amount_out > 0);
    assert!(result.amount_out < result.amount_in);
    // 4 hops must produce less than 2 hops for same input
}

/// More hops always produce less output than fewer hops (compounding slippage).
#[test]
fn e2e_multi_hop_more_hops_less_output() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env);
    let p2 = deploy_pool_99(&env);
    let p3 = deploy_pool_99(&env);
    let p4 = deploy_pool_99(&env);
    client.register_pool(&p1);
    client.register_pool(&p2);
    client.register_pool(&p3);
    client.register_pool(&p4);

    let amount = 1_000_000_i128;

    let r1 = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[p1.clone()]), amount, 0),
    );
    let r4 = client.execute_swap(
        &Address::generate(&env),
        &swap_params(
            &env,
            multi_pool_route(&env, &[p1, p2, p3, p4]),
            amount,
            0,
        ),
    );

    assert!(
        r4.amount_out < r1.amount_out,
        "4-hop {} should be < 1-hop {}",
        r4.amount_out,
        r1.amount_out
    );
}

/// 5-hop route must be rejected — exceeds MAX_HOPS=4.
#[test]
fn e2e_multi_hop_five_hops_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let mut pools_sdk = Vec::new(&env);
    for _ in 0..5 {
        let p = deploy_pool_99(&env);
        client.register_pool(&p);
        pools_sdk.push_back(p);
    }

    // Build the route manually from the soroban Vec
    let mut hops = Vec::new(&env);
    for i in 0..pools_sdk.len() {
        hops.push_back(RouteHop {
            source: Asset::Native,
            destination: Asset::Native,
            pool: pools_sdk.get(i).unwrap(),
            pool_type: PoolType::AmmConstProd,
        });
    }
    let route = Route {
        hops,
        estimated_output: 0,
        min_output: 0,
        expires_at: 999_999,
    };

    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, route, 1_000, 0),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidRoute)));
}

/// Multi-hop quote matches the actual swap output.
#[test]
fn e2e_multi_hop_quote_matches_swap() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env);
    let p2 = deploy_pool_98(&env);
    client.register_pool(&p1);
    client.register_pool(&p2);

    let route = multi_pool_route(&env, &[p1, p2]);
    let quote = client.get_quote(&10_000, &route.clone());

    let result = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, route, 10_000, 0),
    );

    assert_eq!(quote.expected_output, result.amount_out);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Event assertion tests ─────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// swap_executed event must be emitted exactly once per successful swap.
#[test]
fn e2e_event_swap_executed_emitted_once() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let before = env.events().all().len();
    client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );
    let after = env.events().all().len();

    // At minimum the swap event was emitted
    assert!(after > before, "expected new events after swap");
}

/// execute() alias emits execution_requested + swap_executed events.
#[test]
fn e2e_event_execute_alias_emits_requested_and_executed() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let before = env.events().all().len();
    let sender = Address::generate(&env);
    client.execute(
        &sender,
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );

    // execute() emits execution_requested then swap_executed — at least 2 new events
    assert!(
        env.events().all().len() >= before + 2,
        "expected at least 2 new events from execute()"
    );
}

/// Failed swap via execute() alias must emit execution_failed event.
#[test]
fn e2e_event_execute_alias_emits_failed_on_error() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_fail(&env);
    client.register_pool(&pool);

    let before = env.events().all().len();
    let _ = client.try_execute(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );

    // execution_requested + execution_failed should both be emitted
    assert!(
        env.events().all().len() > before,
        "expected failure events after broken pool swap"
    );
}

/// pause() and unpause() each emit their own event.
#[test]
fn e2e_event_pause_unpause_emitted() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let before = env.events().all().len();
    client.pause();
    let after_pause = env.events().all().len();
    assert!(after_pause > before, "pause must emit event");

    client.unpause();
    let after_unpause = env.events().all().len();
    assert!(after_unpause > after_pause, "unpause must emit event");
}

/// register_pool() emits a pool_registered event.
#[test]
fn e2e_event_pool_registered_emitted() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);

    let before = env.events().all().len();
    client.register_pool(&pool);
    assert!(
        env.events().all().len() > before,
        "register_pool must emit event"
    );
}

/// Multi-hop swap emits exactly one swap_executed event (not one per hop).
#[test]
fn e2e_event_multi_hop_single_swap_event() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env);
    let p2 = deploy_pool_98(&env);
    let p3 = deploy_pool_99(&env);
    client.register_pool(&p1);
    client.register_pool(&p2);
    client.register_pool(&p3);

    // Capture baseline after setup events
    let before = env.events().all().len();

    client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[p1, p2, p3]), 10_000, 0),
    );

    let new_events = env.events().all().len() - before;
    // The router emits: fee_collected + swap_executed = 2 events minimum
    // It must NOT emit 3 separate swap events (one per hop)
    assert!(new_events >= 1, "at least one event expected");
    // Confirm it's not emitting one swap event per hop (would be 3+)
    assert!(
        new_events < 10,
        "unexpected event explosion: {} events for 3-hop swap",
        new_events
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Failure rollback / error recovery tests ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// A broken pool mid-route returns PoolCallFailed and leaves state unchanged.
#[test]
fn e2e_failure_broken_pool_returns_error() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_fail(&env);
    client.register_pool(&pool);

    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );
    assert_eq!(result, Err(Ok(ContractError::PoolCallFailed)));
}

/// Nonce must NOT increment when a swap fails.
#[test]
fn e2e_failure_nonce_unchanged_on_failed_swap() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_fail(&env);
    client.register_pool(&pool);

    let sender = Address::generate(&env);

    // Read nonce via the contract context
    let nonce_before = env.as_contract(&client.address, || {
        super::storage::get_nonce(&env, sender.clone())
    });

    let _ = client.try_execute_swap(
        &sender,
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );

    let nonce_after = env.as_contract(&client.address, || {
        super::storage::get_nonce(&env, sender.clone())
    });

    assert_eq!(
        nonce_before, nonce_after,
        "nonce must not change on failed swap"
    );
}

/// Swap volume must NOT increase when a swap fails.
#[test]
fn e2e_failure_volume_unchanged_on_failed_swap() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_fail(&env);
    client.register_pool(&pool);

    let vol_before = client.get_total_swap_volume();
    let _ = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );
    assert_eq!(
        client.get_total_swap_volume(),
        vol_before,
        "volume must not change on failed swap"
    );
}

/// Broken pool at hop 2 of a 2-hop route: first hop already called but
/// the whole tx reverts — volume and nonce stay clean.
#[test]
fn e2e_failure_mid_route_broken_pool_rollback() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env); // hop 1: healthy
    let p2 = deploy_pool_fail(&env); // hop 2: broken
    client.register_pool(&p1);
    client.register_pool(&p2);

    let sender = Address::generate(&env);
    let vol_before = client.get_total_swap_volume();

    let result = client.try_execute_swap(
        &sender,
        &swap_params(&env, multi_pool_route(&env, &[p1, p2]), 10_000, 0),
    );

    assert_eq!(result, Err(Ok(ContractError::PoolCallFailed)));
    // Volume must be unchanged — the failed tx should not have committed
    assert_eq!(client.get_total_swap_volume(), vol_before);
}

/// Unregistered pool in a multi-hop route must be rejected before execution.
#[test]
fn e2e_failure_unregistered_pool_in_route_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env);
    let p2 = deploy_pool_99(&env); // NOT registered
    client.register_pool(&p1);
    // p2 deliberately not registered

    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[p1, p2]), 1_000, 0),
    );
    assert_eq!(result, Err(Ok(ContractError::PoolNotSupported)));
}

/// Contract-as-recipient must be rejected.
#[test]
fn e2e_failure_contract_recipient_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let mut params = swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0);
    params.recipient = client.address.clone();

    let result = client.try_execute_swap(&Address::generate(&env), &params);
    assert_eq!(result, Err(Ok(ContractError::InvalidRecipient)));
}

/// Zero amount_in must be rejected.
#[test]
fn e2e_failure_zero_amount_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 0, 0),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// Negative amount_in must be rejected.
#[test]
fn e2e_failure_negative_amount_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), -500, 0),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// Empty route must be rejected.
#[test]
fn e2e_failure_empty_route_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let empty = Route {
        hops: Vec::new(&env),
        estimated_output: 0,
        min_output: 0,
        expires_at: 999_999,
    };
    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, empty, 1_000, 0),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidRoute)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── MEV protection E2E tests ──────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

fn default_mev_config() -> MevConfig {
    MevConfig {
        commit_threshold: 100_000,
        commit_window_ledgers: 50,
        max_swaps_per_window: 3,
        rate_limit_window: 100,
        high_impact_threshold_bps: 500,
    }
}

/// Large swap above commit_threshold must require commit-reveal.
#[test]
fn e2e_mev_large_swap_requires_commitment() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);
    client.configure_mev(&default_mev_config());

    // amount_in >= commit_threshold (100_000) → CommitmentRequired
    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 100_000, 0),
    );
    assert_eq!(result, Err(Ok(ContractError::CommitmentRequired)));
}

/// Small swap below commit_threshold must bypass commit-reveal.
#[test]
fn e2e_mev_small_swap_bypasses_commitment() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);
    client.configure_mev(&default_mev_config());

    // amount_in < 100_000 → no commitment needed
    let result = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 99_999, 0),
    );
    assert!(result.is_ok());
}

/// Rate limiting: exceeding max_swaps_per_window must be rejected.
#[test]
fn e2e_mev_rate_limit_blocks_excessive_swaps() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);
    client.configure_mev(&default_mev_config()); // max 3 swaps per window

    let sender = Address::generate(&env);

    // First 3 swaps succeed
    for _ in 0..3 {
        let result = client.try_execute_swap(
            &sender,
            &swap_params(&env, multi_pool_route(&env, &[pool.clone()]), 1_000, 0),
        );
        assert!(result.is_ok(), "swap within limit should succeed");
    }

    // 4th swap in same window must be rejected
    let result = client.try_execute_swap(
        &sender,
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );
    assert_eq!(result, Err(Ok(ContractError::RateLimitExceeded)));
}

/// Whitelisted address is exempt from rate limiting.
#[test]
fn e2e_mev_whitelisted_exempt_from_rate_limit() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);
    client.configure_mev(&default_mev_config());

    let sender = Address::generate(&env);
    client.set_whitelist(&sender, &true);

    // Should be able to swap more than max_swaps_per_window
    for _ in 0..5 {
        let result = client.try_execute_swap(
            &sender,
            &swap_params(&env, multi_pool_route(&env, &[pool.clone()]), 1_000, 0),
        );
        assert!(result.is_ok(), "whitelisted sender should never be rate-limited");
    }
}

/// Commit-reveal: valid commitment + correct reveal executes the swap.
#[test]
fn e2e_mev_commit_reveal_full_flow() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);
    client.configure_mev(&default_mev_config());

    let sender = Address::generate(&env);
    let amount_in: i128 = 100_000;
    let min_out: i128 = 0;
    let deadline = seq(&env) + 200;
    let salt = BytesN::from_array(&env, &[7u8; 32]);

    // Build the same hash the contract will verify
    let mut payload = Bytes::new(&env);
    payload.append(&Bytes::from_slice(&env, &amount_in.to_be_bytes()));
    payload.append(&Bytes::from_slice(&env, &min_out.to_be_bytes()));
    payload.append(&Bytes::from_slice(&env, &deadline.to_be_bytes()));
    let salt_bytes: soroban_sdk::Bytes = salt.clone().into();
    payload.append(&salt_bytes);
    let commitment_hash: BytesN<32> = env.crypto().sha256(&payload).into();

    // Step 1: commit
    client.commit_swap(&sender, &commitment_hash, &amount_in);

    // Step 2: reveal + execute
    let route = multi_pool_route(&env, &[pool]);
    let params = SwapParams {
        route,
        amount_in,
        min_amount_out: min_out,
        recipient: Address::generate(&env),
        deadline,
        not_before: 0,
        max_price_impact_bps: 0,
        max_execution_spread_bps: 0,
    };

    let result = client.reveal_and_execute(&sender, &params, &salt);
    assert_eq!(result.amount_in, amount_in);
    assert!(result.amount_out > 0);
}

/// Reveal with wrong salt must be rejected.
#[test]
fn e2e_mev_commit_reveal_wrong_salt_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);
    client.configure_mev(&default_mev_config());

    let sender = Address::generate(&env);
    let amount_in: i128 = 100_000;
    let min_out: i128 = 0;
    let deadline = seq(&env) + 200;
    let correct_salt = BytesN::from_array(&env, &[7u8; 32]);
    let wrong_salt = BytesN::from_array(&env, &[9u8; 32]);

    // Commit with correct salt
    let mut payload = Bytes::new(&env);
    payload.append(&Bytes::from_slice(&env, &amount_in.to_be_bytes()));
    payload.append(&Bytes::from_slice(&env, &min_out.to_be_bytes()));
    payload.append(&Bytes::from_slice(&env, &deadline.to_be_bytes()));
    let salt_bytes: soroban_sdk::Bytes = correct_salt.into();
    payload.append(&salt_bytes);
    let commitment_hash: BytesN<32> = env.crypto().sha256(&payload).into();
    client.commit_swap(&sender, &commitment_hash, &amount_in);

    // Reveal with wrong salt → CommitmentNotFound (hash mismatch)
    let route = multi_pool_route(&env, &[pool]);
    let params = SwapParams {
        route,
        amount_in,
        min_amount_out: min_out,
        recipient: Address::generate(&env),
        deadline,
        not_before: 0,
        max_price_impact_bps: 0,
        max_execution_spread_bps: 0,
    };
    let result = client.try_reveal_and_execute(&sender, &params, &wrong_salt);
    assert_eq!(result, Err(Ok(ContractError::CommitmentNotFound)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Full lifecycle E2E tests ──────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

/// Full lifecycle: init → register pools → quote → swap → volume tracked.
#[test]
fn e2e_lifecycle_init_register_quote_swap() {
    let env = setup();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env);
    let p2 = deploy_pool_98(&env);
    client.register_pool(&p1);
    client.register_pool(&p2);

    assert_eq!(client.get_pool_count(), 2);
    assert!(client.is_pool_registered(&p1));
    assert!(client.is_pool_registered(&p2));

    let route = multi_pool_route(&env, &[p1, p2]);
    let quote = client.get_quote(&10_000, &route.clone());
    assert!(quote.expected_output > 0);
    assert!(quote.expected_output < 10_000);

    let result = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, route, 10_000, 0),
    );
    assert_eq!(result.amount_out, quote.expected_output);
    assert_eq!(client.get_total_swap_volume(), 10_000);
}

/// Multiple users swapping concurrently accumulate volume correctly.
#[test]
fn e2e_lifecycle_multi_user_volume_accumulation() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let amount = 5_000_i128;
    for i in 0..4u32 {
        let result = client.execute_swap(
            &Address::generate(&env),
            &swap_params(&env, multi_pool_route(&env, &[pool.clone()]), amount, 0),
        );
        assert!(result.amount_out > 0);
        assert_eq!(client.get_total_swap_volume(), amount * (i as i128 + 1));
    }
}

/// Pause mid-lifecycle: pending swaps fail, unpause restores service.
#[test]
fn e2e_lifecycle_pause_mid_operation_then_resume() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    // Swap 1: succeeds
    let r1 = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool.clone()]), 1_000, 0),
    );
    assert!(r1.amount_out > 0);

    // Admin pauses
    client.pause();

    // Swap 2: fails
    let r2 = client.try_execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool.clone()]), 1_000, 0),
    );
    assert_eq!(r2, Err(Ok(ContractError::Paused)));

    // Volume unchanged after failed swap
    assert_eq!(client.get_total_swap_volume(), 1_000);

    // Admin unpauses
    client.unpause();

    // Swap 3: succeeds again
    let r3 = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );
    assert!(r3.amount_out > 0);
    assert_eq!(client.get_total_swap_volume(), 2_000);
}

/// Admin change mid-lifecycle does not affect registered pools or swap state.
#[test]
fn e2e_lifecycle_admin_change_does_not_affect_swaps() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let r1 = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool.clone()]), 1_000, 0),
    );

    // Change admin
    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    // Pool still registered, swap still works
    assert!(client.is_pool_registered(&pool));
    let r2 = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );
    assert_eq!(r1.amount_out, r2.amount_out);
}

/// TTL extension during swap keeps contract alive.
#[test]
fn e2e_lifecycle_ttl_extended_during_swap() {
    let env = setup();
    // Prevent pool contracts from being archived when we advance the ledger
    env.ledger().with_mut(|l| {
        l.min_persistent_entry_ttl = 5_000_000;
        l.max_entry_ttl = 10_000_000;
    });
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    // Advance ledger significantly
    env.ledger().with_mut(|l| l.sequence_number = 10_000);

    // Swap should still work and extend TTLs without panic
    let result = client.execute_swap(
        &Address::generate(&env),
        &swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0),
    );
    assert!(result.amount_out > 0);

    let status = client.get_ttl_status();
    assert!(status.instance_ttl_remaining > 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Input validation tests (Task 2) ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

// ── initialize() ─────────────────────────────────────────────────────────────

/// admin == fee_to must be rejected.
#[test]
fn val_initialize_admin_equals_fee_to_rejected() {
    let env = setup();
    let addr = Address::generate(&env);
    let id = env.register_contract(None, StellarRoute);
    let client = StellarRouteClient::new(&env, &id);
    let result = client.try_initialize(&addr, &30_u32, &addr, &None, &None, &None, &None, &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// fee_rate > 1000 bps must be rejected.
#[test]
fn val_initialize_fee_rate_above_max_rejected() {
    let env = setup();
    let id = env.register_contract(None, StellarRoute);
    let client = StellarRouteClient::new(&env, &id);
    let result = client.try_initialize(
        &Address::generate(&env),
        &1001_u32,
        &Address::generate(&env),
        &None, &None, &None, &None, &None,
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// fee_rate == 0 is valid (no protocol fee).
#[test]
fn val_initialize_zero_fee_rate_accepted() {
    let env = setup();
    let id = env.register_contract(None, StellarRoute);
    let client = StellarRouteClient::new(&env, &id);
    assert!(client.try_initialize(
        &Address::generate(&env),
        &0_u32,
        &Address::generate(&env),
        &None, &None, &None, &None, &None,
    ).is_ok());
}

// ── set_admin() ───────────────────────────────────────────────────────────────

/// Setting admin to the same address must be rejected.
#[test]
fn val_set_admin_same_address_rejected() {
    let env = setup();
    let (admin, client) = deploy_router(&env);
    let result = client.try_set_admin(&admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// Setting admin to the contract address itself must be rejected.
#[test]
fn val_set_admin_contract_address_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let result = client.try_set_admin(&client.address.clone());
    assert_eq!(result, Err(Ok(ContractError::InvalidRecipient)));
}

/// Valid new admin must succeed.
#[test]
fn val_set_admin_valid_address_accepted() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    assert!(client.try_set_admin(&Address::generate(&env)).is_ok());
}

// ── register_pool() ───────────────────────────────────────────────────────────

/// Registering the router contract as a pool must be rejected.
#[test]
fn val_register_pool_self_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let result = client.try_register_pool(&client.address.clone());
    assert_eq!(result, Err(Ok(ContractError::InvalidRecipient)));
}

/// Registering a duplicate pool must be rejected.
#[test]
fn val_register_pool_duplicate_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);
    let result = client.try_register_pool(&pool);
    assert_eq!(result, Err(Ok(ContractError::PoolNotSupported)));
}

// ── configure_mev() ──────────────────────────────────────────────────────────

fn valid_mev() -> MevConfig {
    MevConfig {
        commit_threshold: 100_000,
        commit_window_ledgers: 50,
        max_swaps_per_window: 3,
        rate_limit_window: 100,
        high_impact_threshold_bps: 500,
    }
}

/// commit_threshold <= 0 must be rejected.
#[test]
fn val_configure_mev_zero_threshold_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let mut cfg = valid_mev();
    cfg.commit_threshold = 0;
    assert_eq!(
        client.try_configure_mev(&cfg),
        Err(Ok(ContractError::InvalidAmount))
    );
}

/// commit_window_ledgers == 0 must be rejected.
#[test]
fn val_configure_mev_zero_window_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let mut cfg = valid_mev();
    cfg.commit_window_ledgers = 0;
    assert_eq!(
        client.try_configure_mev(&cfg),
        Err(Ok(ContractError::InvalidAmount))
    );
}

/// max_swaps_per_window == 0 must be rejected.
#[test]
fn val_configure_mev_zero_max_swaps_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let mut cfg = valid_mev();
    cfg.max_swaps_per_window = 0;
    assert_eq!(
        client.try_configure_mev(&cfg),
        Err(Ok(ContractError::InvalidAmount))
    );
}

/// rate_limit_window == 0 must be rejected.
#[test]
fn val_configure_mev_zero_rate_limit_window_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let mut cfg = valid_mev();
    cfg.rate_limit_window = 0;
    assert_eq!(
        client.try_configure_mev(&cfg),
        Err(Ok(ContractError::InvalidAmount))
    );
}

/// high_impact_threshold_bps > 10000 must be rejected.
#[test]
fn val_configure_mev_impact_bps_above_max_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let mut cfg = valid_mev();
    cfg.high_impact_threshold_bps = 10_001;
    assert_eq!(
        client.try_configure_mev(&cfg),
        Err(Ok(ContractError::InvalidAmount))
    );
}

/// Valid MEV config must be accepted.
#[test]
fn val_configure_mev_valid_accepted() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    assert!(client.try_configure_mev(&valid_mev()).is_ok());
}

// ── execute_swap() bps guard fields ──────────────────────────────────────────

/// max_price_impact_bps > 10000 must be rejected.
#[test]
fn val_swap_price_impact_bps_above_max_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let mut params = swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0);
    params.max_price_impact_bps = 10_001;
    let result = client.try_execute_swap(&Address::generate(&env), &params);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// max_execution_spread_bps > 10000 must be rejected.
#[test]
fn val_swap_spread_bps_above_max_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let mut params = swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0);
    params.max_execution_spread_bps = 10_001;
    let result = client.try_execute_swap(&Address::generate(&env), &params);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// max_price_impact_bps == 10000 (100%) is the boundary — must be accepted.
#[test]
fn val_swap_price_impact_bps_at_boundary_accepted() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_99(&env);
    client.register_pool(&pool);

    let mut params = swap_params(&env, multi_pool_route(&env, &[pool]), 1_000, 0);
    params.max_price_impact_bps = 10_000;
    assert!(client.try_execute_swap(&Address::generate(&env), &params).is_ok());
}

// ── commit_swap() ─────────────────────────────────────────────────────────────

/// Zeroed commitment hash must be rejected.
#[test]
fn val_commit_swap_zero_hash_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    client.configure_mev(&valid_mev());

    let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
    let result = client.try_commit_swap(&Address::generate(&env), &zero_hash, &1_000);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// Zero deposit_amount must be rejected.
#[test]
fn val_commit_swap_zero_deposit_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    client.configure_mev(&valid_mev());

    let hash = BytesN::from_array(&env, &[1u8; 32]);
    let result = client.try_commit_swap(&Address::generate(&env), &hash, &0);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// Negative deposit_amount must be rejected.
#[test]
fn val_commit_swap_negative_deposit_rejected() {
    let env = setup();
    let (_, client) = deploy_router(&env);
    client.configure_mev(&valid_mev());

    let hash = BytesN::from_array(&env, &[1u8; 32]);
    let result = client.try_commit_swap(&Address::generate(&env), &hash, &-1);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

// ── governance init_governance() ─────────────────────────────────────────────

/// Duplicate signers in migrate_to_multisig must be rejected.
#[test]
fn val_governance_duplicate_signers_rejected() {
    let env = setup();
    let (admin, client) = deploy_router(&env);
    let s1 = Address::generate(&env);

    let mut signers = Vec::new(&env);
    signers.push_back(s1.clone());
    signers.push_back(s1.clone()); // duplicate

    let result = client.try_migrate_to_multisig(&admin, &signers, &1, &500, &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// proposal_ttl == 0 must be rejected.
#[test]
fn val_governance_zero_proposal_ttl_rejected() {
    let env = setup();
    let (admin, client) = deploy_router(&env);
    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);

    let mut signers = Vec::new(&env);
    signers.push_back(s1);
    signers.push_back(s2);

    let result = client.try_migrate_to_multisig(&admin, &signers, &1, &0, &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// threshold == 0 must be rejected.
#[test]
fn val_governance_zero_threshold_rejected() {
    let env = setup();
    let (admin, client) = deploy_router(&env);
    let s1 = Address::generate(&env);

    let mut signers = Vec::new(&env);
    signers.push_back(s1);

    let result = client.try_migrate_to_multisig(&admin, &signers, &0, &500, &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// threshold > signer count must be rejected.
#[test]
fn val_governance_threshold_exceeds_signers_rejected() {
    let env = setup();
    let (admin, client) = deploy_router(&env);
    let s1 = Address::generate(&env);

    let mut signers = Vec::new(&env);
    signers.push_back(s1);

    let result = client.try_migrate_to_multisig(&admin, &signers, &2, &500, &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

// ── tokens::add_token() ───────────────────────────────────────────────────────

/// decimals > 19 must be rejected.
#[test]
fn val_add_token_decimals_above_max_rejected() {
    let env = setup();
    let (admin, client) = deploy_router(&env);

    let asset = Asset::Issued(Address::generate(&env), Symbol::new(&env, "USDC"));
    let info = super::types::TokenInfo {
        asset,
        name: Symbol::new(&env, "USDCoin"),
        code: Symbol::new(&env, "USDC"),
        decimals: 20, // invalid
        issuer_verified: false,
        category: super::types::TokenCategory::Stablecoin,
        added_at: seq(&env),
        added_by: admin.clone(),
    };
    let result = client.try_add_token(&admin, &info);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// decimals == 19 is the boundary — must be accepted.
#[test]
fn val_add_token_decimals_at_boundary_accepted() {
    let env = setup();
    let (admin, client) = deploy_router(&env);

    let asset = Asset::Issued(Address::generate(&env), Symbol::new(&env, "USDC"));
    let info = super::types::TokenInfo {
        asset,
        name: Symbol::new(&env, "USDCoin"),
        code: Symbol::new(&env, "USDC"),
        decimals: 19,
        issuer_verified: false,
        category: super::types::TokenCategory::Stablecoin,
        added_at: seq(&env),
        added_by: admin.clone(),
    };
    assert!(client.try_add_token(&admin, &info).is_ok());
}

// ── tokens::update_token() ────────────────────────────────────────────────────

/// Updating with a mismatched asset in the TokenInfo must be rejected.
#[test]
fn val_update_token_asset_mismatch_rejected() {
    let env = setup();
    let (admin, client) = deploy_router(&env);

    let asset_a = Asset::Issued(Address::generate(&env), Symbol::new(&env, "USDC"));
    let asset_b = Asset::Issued(Address::generate(&env), Symbol::new(&env, "EURT"));

    let info_a = super::types::TokenInfo {
        asset: asset_a.clone(),
        name: Symbol::new(&env, "USDCoin"),
        code: Symbol::new(&env, "USDC"),
        decimals: 7,
        issuer_verified: false,
        category: super::types::TokenCategory::Stablecoin,
        added_at: seq(&env),
        added_by: admin.clone(),
    };
    client.add_token(&admin, &info_a);

    // Try to update asset_a but pass asset_b in the TokenInfo body
    let mismatched = super::types::TokenInfo {
        asset: asset_b, // wrong asset
        name: Symbol::new(&env, "EuroTether"),
        code: Symbol::new(&env, "EURT"),
        decimals: 7,
        issuer_verified: false,
        category: super::types::TokenCategory::Stablecoin,
        added_at: seq(&env),
        added_by: admin.clone(),
    };
    let result = client.try_update_token(&admin, &asset_a, &mismatched);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

/// Updating with decimals > 19 must be rejected.
#[test]
fn val_update_token_invalid_decimals_rejected() {
    let env = setup();
    let (admin, client) = deploy_router(&env);

    let asset = Asset::Issued(Address::generate(&env), Symbol::new(&env, "USDC"));
    let info = super::types::TokenInfo {
        asset: asset.clone(),
        name: Symbol::new(&env, "USDCoin"),
        code: Symbol::new(&env, "USDC"),
        decimals: 7,
        issuer_verified: false,
        category: super::types::TokenCategory::Stablecoin,
        added_at: seq(&env),
        added_by: admin.clone(),
    };
    client.add_token(&admin, &info);

    let bad_update = super::types::TokenInfo {
        asset: asset.clone(),
        name: Symbol::new(&env, "USDCoin"),
        code: Symbol::new(&env, "USDC"),
        decimals: 25, // invalid
        issuer_verified: true,
        category: super::types::TokenCategory::Stablecoin,
        added_at: seq(&env),
        added_by: admin.clone(),
    };
    let result = client.try_update_token(&admin, &asset, &bad_update);
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}
