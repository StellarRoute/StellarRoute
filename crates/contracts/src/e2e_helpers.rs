//! Shared harness helpers — imported by e2e_harness.rs tests.

use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, Vec};

use super::{
    router::{StellarRoute, StellarRouteClient},
    types::{Asset, PoolType, Route, RouteHop, SwapParams},
};

use super::e2e_harness::{deploy_pool_98, deploy_pool_99, deploy_pool_fail};

pub fn setup() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

pub fn deploy_router(env: &Env) -> (Address, StellarRouteClient<'_>) {
    let admin = Address::generate(env);
    let fee_to = Address::generate(env);
    let id = env.register_contract(None, StellarRoute);
    let client = StellarRouteClient::new(env, &id);
    client.initialize(&admin, &30_u32, &fee_to, &None, &None, &None, &None, &None);
    (admin, client)
}

pub fn seq(env: &Env) -> u64 {
    env.ledger().sequence() as u64
}

pub fn multi_pool_route(env: &Env, pools: &[Address]) -> Route {
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

pub fn swap_params(env: &Env, route: Route, amount_in: i128, min_out: i128) -> SwapParams {
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
