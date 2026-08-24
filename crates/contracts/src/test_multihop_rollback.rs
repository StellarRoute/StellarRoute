//! Multi-hop CPI failure atomic rollback integration tests.
//!
//! Verifies that failed multi-hop swaps do not commit partial state changes
//! (volume, nonce) and that failures propagate from adapter CPI calls.

use crate::errors::ContractError;
use crate::router::{StellarRoute, StellarRouteClient};
use crate::types::{Asset, PoolType, Route, RouteHop, SwapParams};
use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, Env, Vec,
};

mod mock_amm {
    use super::Asset;
    use soroban_sdk::{contract, contractimpl, Env};

    #[contract]
    pub struct MockAmmPool;

    #[contractimpl]
    impl MockAmmPool {
        pub fn adapter_quote(_e: Env, _in: Asset, _out: Asset, amount_in: i128) -> i128 {
            amount_in * 99 / 100
        }

        pub fn swap(_e: Env, _in: Asset, _out: Asset, amount_in: i128, min_out: i128) -> i128 {
            let out = amount_in * 99 / 100;
            if out < min_out {
                panic!("mock pool: slippage");
            }
            out
        }

        pub fn get_rsrvs(_e: Env) -> (i128, i128) {
            (1_000_000_000, 1_000_000_000)
        }
    }
}

mod mock_failing {
    use super::Asset;
    use soroban_sdk::{contract, contractimpl, Env};

    #[contract]
    pub struct MockFailingPool;

    #[contractimpl]
    impl MockFailingPool {
        pub fn adapter_quote(_e: Env, _in: Asset, _out: Asset, _amount: i128) -> i128 {
            panic!("mock: pool unavailable")
        }

        pub fn swap(_e: Env, _in: Asset, _out: Asset, _amount: i128, _min: i128) -> i128 {
            panic!("mock: pool unavailable")
        }

        pub fn get_rsrvs(_e: Env) -> (i128, i128) {
            panic!("mock: pool unavailable")
        }
    }
}

fn setup_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn deploy_router(env: &Env) -> (Address, StellarRouteClient<'_>) {
    let admin = Address::generate(env);
    let fee_to = Address::generate(env);
    let id = env.register_contract(None, StellarRoute);
    let client = StellarRouteClient::new(env, &id);
    client.initialize(&admin, &30, &fee_to, &None, &None, &None, &None, &None);
    (admin, client)
}

fn deploy_pool_99(env: &Env) -> Address {
    env.register_contract(None, mock_amm::MockAmmPool)
}

fn deploy_pool_fail(env: &Env) -> Address {
    env.register_contract(None, mock_failing::MockFailingPool)
}

fn seq(env: &Env) -> u64 {
    env.ledger().sequence() as u64
}

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

fn swap_params(
    env: &Env,
    route: Route,
    amount_in: i128,
    min_out: i128,
    recipient: Address,
) -> SwapParams {
    SwapParams {
        route,
        amount_in,
        min_amount_out: min_out,
        recipient,
        deadline: seq(env) + 200,
        not_before: 0,
        max_price_impact_bps: 0,
        max_execution_spread_bps: 0,
    }
}

#[test]
fn test_single_hop_failure_rolls_back() {
    let env = setup_env();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_fail(&env);
    client.register_pool(&pool);

    let sender = Address::generate(&env);
    let events_before = env.events().all().len();
    let result = client.try_execute_swap(
        &sender,
        &swap_params(
            &env,
            multi_pool_route(&env, &[pool]),
            100,
            0,
            sender.clone(),
        ),
    );

    assert_eq!(result, Err(Ok(ContractError::AmmSwapCallFailed)));
    assert_eq!(
        env.events().all().len(),
        events_before,
        "failed swap should not emit new events"
    );
}

#[test]
fn test_failure_at_hop_index_0() {
    let env = setup_env();
    let (_, client) = deploy_router(&env);

    let failing = deploy_pool_fail(&env);
    let healthy = deploy_pool_99(&env);
    client.register_pool(&failing);
    client.register_pool(&healthy);

    let sender = Address::generate(&env);
    let vol_before = client.get_total_swap_volume();

    let result = client.try_execute_swap(
        &sender,
        &swap_params(
            &env,
            multi_pool_route(&env, &[failing, healthy]),
            100,
            0,
            sender.clone(),
        ),
    );

    assert_eq!(result, Err(Ok(ContractError::AmmSwapCallFailed)));
    assert_eq!(client.get_total_swap_volume(), vol_before);
}

#[test]
fn test_failure_at_hop_index_1() {
    let env = setup_env();
    let (_, client) = deploy_router(&env);

    let healthy = deploy_pool_99(&env);
    let failing = deploy_pool_fail(&env);
    client.register_pool(&healthy);
    client.register_pool(&failing);

    let sender = Address::generate(&env);
    let vol_before = client.get_total_swap_volume();

    let result = client.try_execute_swap(
        &sender,
        &swap_params(
            &env,
            multi_pool_route(&env, &[healthy, failing]),
            100,
            0,
            sender.clone(),
        ),
    );

    assert_eq!(result, Err(Ok(ContractError::AmmSwapCallFailed)));
    assert_eq!(
        client.get_total_swap_volume(),
        vol_before,
        "mid-route failure must roll back committed volume"
    );
}

#[test]
fn test_failure_at_hop_index_2_three_hop() {
    let env = setup_env();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env);
    let p2 = deploy_pool_99(&env);
    let p3 = deploy_pool_fail(&env);
    client.register_pool(&p1);
    client.register_pool(&p2);
    client.register_pool(&p3);

    let sender = Address::generate(&env);
    let vol_before = client.get_total_swap_volume();

    let result = client.try_execute_swap(
        &sender,
        &swap_params(
            &env,
            multi_pool_route(&env, &[p1, p2, p3]),
            100,
            0,
            sender.clone(),
        ),
    );

    assert_eq!(result, Err(Ok(ContractError::AmmSwapCallFailed)));
    assert_eq!(client.get_total_swap_volume(), vol_before);
}

#[test]
fn test_adapter_contract_failure_propagates() {
    let env = setup_env();
    let (_, client) = deploy_router(&env);
    let pool = deploy_pool_fail(&env);
    client.register_pool(&pool);

    let sender = Address::generate(&env);
    let result = client.try_execute_swap(
        &sender,
        &swap_params(
            &env,
            multi_pool_route(&env, &[pool]),
            100,
            0,
            sender.clone(),
        ),
    );

    assert_eq!(result, Err(Ok(ContractError::AmmSwapCallFailed)));
}

#[test]
fn test_all_hops_succeed_no_rollback() {
    let env = setup_env();
    let (_, client) = deploy_router(&env);

    let p1 = deploy_pool_99(&env);
    let p2 = deploy_pool_99(&env);
    client.register_pool(&p1);
    client.register_pool(&p2);

    let sender = Address::generate(&env);
    let vol_before = client.get_total_swap_volume();
    let events_before = env.events().all().len();

    let result = client.execute_swap(
        &sender,
        &swap_params(
            &env,
            multi_pool_route(&env, &[p1, p2]),
            100,
            0,
            sender.clone(),
        ),
    );

    assert!(result.amount_out > 0);
    assert!(client.get_total_swap_volume() > vol_before);
    assert!(env.events().all().len() > events_before);
}
