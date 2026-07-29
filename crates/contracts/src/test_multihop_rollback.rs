//! Multi-hop CPI failure atomic rollback integration tests

#![cfg(test)]

use soroban_sdk::{
    Address, Env, Vec as SorobanVec,
};

use crate::router::{StellarRoute, StellarRouteClient};
use crate::types::{Asset, PoolType, Route, RouteHop, SwapParams};

mod failing_pool {
    use crate::types::Asset;
    use soroban_sdk::{contract, contractimpl, Env};

    #[contract]
    pub struct FailingAdapter;

    #[contractimpl]
    impl FailingAdapter {
        pub fn swap(
            _e: Env,
            _in_asset: Asset,
            _out_asset: Asset,
            _amount_in: i128,
            _min_out: i128,
        ) -> i128 {
            panic!("mock failing: insufficient liquidity")
        }

        pub fn adapter_quote(
            _e: Env,
            _in_asset: Asset,
            _out_asset: Asset,
            amount_in: i128,
        ) -> i128 {
            amount_in
        }

        pub fn get_rsrvs(_e: Env) -> (i128, i128) {
            (1_000_000_000, 1_000_000_000)
        }
    }
}

mod success_pool {
    use crate::types::Asset;
    use soroban_sdk::{contract, contractimpl, Env};

    #[contract]
    pub struct SuccessAdapter;

    #[contractimpl]
    impl SuccessAdapter {
        pub fn swap(
            _e: Env,
            _in_asset: Asset,
            _out_asset: Asset,
            amount_in: i128,
            _min_out: i128,
        ) -> i128 {
            amount_in
        }

        pub fn adapter_quote(
            _e: Env,
            _in_asset: Asset,
            _out_asset: Asset,
            amount_in: i128,
        ) -> i128 {
            amount_in
        }

        pub fn get_rsrvs(_e: Env) -> (i128, i128) {
            (1_000_000_000, 1_000_000_000)
        }
    }
}

use failing_pool::FailingAdapter;
use success_pool::SuccessAdapter;

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};
    use soroban_sdk::TryFromVal;

    fn setup_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn deploy_router(env: &Env) -> StellarRouteClient<'_> {
        let admin = Address::generate(env);
        let fee_to = Address::generate(env);
        let id = env.register_contract(None, StellarRoute);
        let client = StellarRouteClient::new(env, &id);
        client.initialize(&admin, &30_u32, &fee_to, &None, &None, &None, &None, &None);
        client
    }

    fn make_route(env: &Env, hops_config: SorobanVec<(Address, PoolType)>) -> Route {
        let mut hops = SorobanVec::new(env);
        for i in 0..hops_config.len() {
            let (pool, pool_type) = hops_config.get(i).unwrap();
            hops.push_back(RouteHop {
                source: Asset::Native,
                destination: Asset::Native,
                pool,
                pool_type,
            });
        }
        Route {
            hops,
            estimated_output: 0,
            min_output: 0,
            expires_at: 99_999,
        }
    }

    fn swap_params(env: &Env, route: Route, amount_in: i128) -> SwapParams {
        SwapParams {
            route,
            amount_in,
            min_amount_out: 0,
            recipient: Address::generate(env),
            deadline: 99_999,
            not_before: 0,
            max_price_impact_bps: 0,
            max_execution_spread_bps: 0,
        }
    }

    #[test]
    fn test_single_hop_failure_rolls_back() {
        let env = setup_env();
        let client = deploy_router(&env);

        let failing_pool = env.register_contract(None, FailingAdapter);
        client.register_pool(&failing_pool);

        let user = Address::generate(&env);
        let mut hops = SorobanVec::new(&env);
        hops.push_back((failing_pool, PoolType::AmmConstProd));
        let route = make_route(&env, hops);
        let params = swap_params(&env, route, 100);

        let result = client.try_execute_swap(&user, &params);
        assert!(result.is_err());

        // Verify no events were emitted for successful swap
        let events = env.events().all();
        let swap_events_count = events
            .iter()
            .filter(|e| {
                if let Some(val) = e.1.get(0) {
                    if let Ok(sym) = soroban_sdk::Symbol::try_from_val(&env, &val) {
                        return sym == soroban_sdk::Symbol::new(&env, "swap_complete");
                    }
                }
                false
            })
            .count();

        assert_eq!(swap_events_count, 0);
    }

    #[test]
    fn test_failure_at_hop_index_0() {
        let env = setup_env();
        let client = deploy_router(&env);

        let failing_pool = env.register_contract(None, FailingAdapter);
        let success_pool = env.register_contract(None, SuccessAdapter);
        client.register_pool(&failing_pool);
        client.register_pool(&success_pool);

        let user = Address::generate(&env);
        let mut hops = SorobanVec::new(&env);
        hops.push_back((failing_pool, PoolType::AmmConstProd));
        hops.push_back((success_pool, PoolType::AmmConstProd));
        let route = make_route(&env, hops);
        let params = swap_params(&env, route, 100);

        let result = client.try_execute_swap(&user, &params);
        assert!(result.is_err());

        // Verify rollback / execution failed event
        let events = env.events().all();
        let rollback_events_count = events
            .iter()
            .filter(|e| {
                if let Some(val) = e.1.get(0) {
                    if let Ok(sym) = soroban_sdk::Symbol::try_from_val(&env, &val) {
                        return sym == soroban_sdk::Symbol::new(&env, "execution_failed");
                    }
                }
                false
            })
            .count();

        assert!(rollback_events_count > 0);
    }

    #[test]
    fn test_failure_at_hop_index_1() {
        let env = setup_env();
        let client = deploy_router(&env);

        let failing_pool = env.register_contract(None, FailingAdapter);
        let success_pool = env.register_contract(None, SuccessAdapter);
        client.register_pool(&failing_pool);
        client.register_pool(&success_pool);

        let user = Address::generate(&env);
        let mut hops = SorobanVec::new(&env);
        hops.push_back((success_pool, PoolType::AmmConstProd));
        hops.push_back((failing_pool, PoolType::AmmConstProd));
        let route = make_route(&env, hops);
        let params = swap_params(&env, route, 100);

        let result = client.try_execute_swap(&user, &params);
        assert!(result.is_err());

        let events = env.events().all();
        let swap_complete_events_count = events
            .iter()
            .filter(|e| {
                if let Some(val) = e.1.get(0) {
                    if let Ok(sym) = soroban_sdk::Symbol::try_from_val(&env, &val) {
                        return sym == soroban_sdk::Symbol::new(&env, "swap_complete");
                    }
                }
                false
            })
            .count();

        assert_eq!(swap_complete_events_count, 0);
    }

    #[test]
    fn test_failure_at_hop_index_2_three_hop() {
        let env = setup_env();
        let client = deploy_router(&env);

        let failing_pool = env.register_contract(None, FailingAdapter);
        let success_pool = env.register_contract(None, SuccessAdapter);
        client.register_pool(&failing_pool);
        client.register_pool(&success_pool);

        let user = Address::generate(&env);
        let mut hops = SorobanVec::new(&env);
        hops.push_back((success_pool.clone(), PoolType::AmmConstProd));
        hops.push_back((success_pool, PoolType::AmmConstProd));
        hops.push_back((failing_pool, PoolType::AmmConstProd));
        let route = make_route(&env, hops);
        let params = swap_params(&env, route, 100);

        let result = client.try_execute_swap(&user, &params);
        assert!(result.is_err());

        let events = env.events().all();
        let rollback_events_count = events
            .iter()
            .filter(|e| {
                if let Some(val) = e.1.get(0) {
                    if let Ok(sym) = soroban_sdk::Symbol::try_from_val(&env, &val) {
                        return sym == soroban_sdk::Symbol::new(&env, "execution_failed");
                    }
                }
                false
            })
            .count();

        assert!(rollback_events_count > 0);
    }

    #[test]
    fn test_adapter_contract_failure_propagates() {
        let env = setup_env();
        let client = deploy_router(&env);

        let failing_pool = env.register_contract(None, FailingAdapter);
        client.register_pool(&failing_pool);

        let user = Address::generate(&env);
        let mut hops = SorobanVec::new(&env);
        hops.push_back((failing_pool, PoolType::AmmConstProd));
        let route = make_route(&env, hops);
        let params = swap_params(&env, route, 100);

        let result = client.try_execute_swap(&user, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_all_hops_succeed_no_rollback() {
        let env = setup_env();
        let client = deploy_router(&env);

        let success_pool = env.register_contract(None, SuccessAdapter);
        client.register_pool(&success_pool);

        let user = Address::generate(&env);
        let mut hops = SorobanVec::new(&env);
        hops.push_back((success_pool.clone(), PoolType::AmmConstProd));
        hops.push_back((success_pool, PoolType::AmmConstProd));
        let route = make_route(&env, hops);
        let params = swap_params(&env, route, 100);

        let result = client.try_execute_swap(&user, &params);
        assert!(result.is_ok());

        let events = env.events().all();
        let rollback_events_count = events
            .iter()
            .filter(|e| {
                if let Some(val) = e.1.get(0) {
                    if let Ok(sym) = soroban_sdk::Symbol::try_from_val(&env, &val) {
                        return sym == soroban_sdk::Symbol::new(&env, "execution_failed");
                    }
                }
                false
            })
            .count();

        assert_eq!(rollback_events_count, 0);
    }
}
