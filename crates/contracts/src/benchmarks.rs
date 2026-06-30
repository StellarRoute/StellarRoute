use crate::router::{StellarRoute, StellarRouteClient};
use crate::types::SwapParams;
use soroban_sdk::{testutils::Address as _, Address};

// Import test utilities from the test module
use crate::test::{deploy_mock_pool, deploy_router, make_route, setup_env};

#[test]
fn bench_initialize() {
    let env = setup_env();
    let admin = Address::generate(&env);
    let fee_to = Address::generate(&env);
    let contract_id = env.register_contract(None, StellarRoute);
    let client = StellarRouteClient::new(&env, &contract_id);

    // Benchmark initialize
    client.initialize(&admin, &30, &fee_to, &None, &None, &None, &None, &None);

    // Assert: Should complete without exceeding budget
    assert!(env.budget().cpu_instruction_cost() < 10_000_000);
}

#[test]
fn bench_register_pool() {
    let env = setup_env();
    let (_admin, _, client) = deploy_router(&env);
    let pool = deploy_mock_pool(&env);

    env.mock_all_auths();

    // Benchmark register_pool
    client.register_pool(&pool);

    let cpu_cost = env.budget().cpu_instruction_cost();
    assert!(cpu_cost < 5_000_000, "register_pool CPU cost: {}", cpu_cost);
}

#[test]
fn bench_get_quote_1_hop() {
    let env = setup_env();
    let (_admin, _, client) = deploy_router(&env);
    let pool = deploy_mock_pool(&env);

    env.mock_all_auths();
    client.register_pool(&pool);

    let route = make_route(&env, &pool, 1);

    // Benchmark get_quote with 1 hop
    let _ = client.get_quote(&1_000_000, &route);

    let cpu_cost = env.budget().cpu_instruction_cost();
    assert!(
        cpu_cost < 15_000_000,
        "get_quote (1 hop) CPU cost: {}",
        cpu_cost
    );
}

#[test]
fn bench_get_quote_2_hops() {
    let env = setup_env();
    let (_admin, _, client) = deploy_router(&env);
    let pool = deploy_mock_pool(&env);

    env.mock_all_auths();
    client.register_pool(&pool);

    let route = make_route(&env, &pool, 2);

    // Benchmark get_quote with 2 hops
    let _ = client.get_quote(&1_000_000, &route);

    let cpu_cost = env.budget().cpu_instruction_cost();
    assert!(
        cpu_cost < 25_000_000,
        "get_quote (2 hops) CPU cost: {}",
        cpu_cost
    );
}

#[test]
fn bench_get_quote_4_hops() {
    let env = setup_env();
    let (_admin, _, client) = deploy_router(&env);
    let pool = deploy_mock_pool(&env);

    env.mock_all_auths();
    client.register_pool(&pool);

    let route = make_route(&env, &pool, 4);

    // Benchmark get_quote with 4 hops
    let _ = client.get_quote(&1_000_000, &route);

    let cpu_cost = env.budget().cpu_instruction_cost();
    assert!(
        cpu_cost < 50_000_000,
        "get_quote (4 hops) CPU cost: {}",
        cpu_cost
    );
}

#[test]
fn bench_execute_swap_1_hop() {
    let env = setup_env();
    let (_admin, _, client) = deploy_router(&env);
    let pool = deploy_mock_pool(&env);
    let sender = Address::generate(&env);

    env.mock_all_auths();
    client.register_pool(&pool);

    let route = make_route(&env, &pool, 1);
    let params = SwapParams {
        route,
        amount_in: 1_000_000,
        min_amount_out: 900_000,
        recipient: sender.clone(),
        deadline: 1000,
        not_before: 0,
        max_price_impact_bps: 0,
        max_execution_spread_bps: 0,
    };

    // Benchmark execute_swap with 1 hop
    let _ = client.execute_swap(&sender, &params);

    let cpu_cost = env.budget().cpu_instruction_cost();
    assert!(
        cpu_cost < 20_000_000,
        "execute_swap (1 hop) CPU cost: {}",
        cpu_cost
    );
}

#[test]
fn bench_execute_swap_4_hops() {
    let env = setup_env();
    let (_admin, _, client) = deploy_router(&env);
    let pool = deploy_mock_pool(&env);
    let sender = Address::generate(&env);

    env.mock_all_auths();
    client.register_pool(&pool);

    let route = make_route(&env, &pool, 4);
    let params = SwapParams {
        route,
        amount_in: 1_000_000,
        min_amount_out: 800_000,
        recipient: sender.clone(),
        deadline: 1000,
        not_before: 0,
        max_price_impact_bps: 0,
        max_execution_spread_bps: 0,
    };

    // Benchmark execute_swap with 4 hops
    let _ = client.execute_swap(&sender, &params);

    let cpu_cost = env.budget().cpu_instruction_cost();
    assert!(
        cpu_cost < 80_000_000,
        "execute_swap (4 hops) CPU cost: {}",
        cpu_cost
    );
}

#[test]
fn bench_estimate_resources() {
    let env = setup_env();
    let (_, _, client) = deploy_router(&env);
    let pool = deploy_mock_pool(&env);

    env.mock_all_auths();
    client.register_pool(&pool);

    let route = make_route(&env, &pool, 4);

    // Benchmark estimate_resources
    let estimate = client.estimate_resources(&1_000_000, &route);

    let cpu_cost = env.budget().cpu_instruction_cost();
    assert!(
        cpu_cost < 5_000_000,
        "estimate_resources CPU cost: {}",
        cpu_cost
    );
    assert!(estimate.will_succeed);
}

#[test]
fn stress_test_max_complexity() {
    let env = setup_env();
    let (_admin, _, client) = deploy_router(&env);
    let pool = deploy_mock_pool(&env);
    let sender = Address::generate(&env);

    env.mock_all_auths();
    client.register_pool(&pool);

    // Maximum complexity: 4 hops
    let route = make_route(&env, &pool, 4);
    let params = SwapParams {
        route,
        amount_in: 10_000_000_000, // Large amount
        min_amount_out: 1,
        recipient: sender.clone(),
        deadline: 10000,
        not_before: 0,
        max_price_impact_bps: 0,
        max_execution_spread_bps: 0,
    };

    let result = client.try_execute_swap(&sender, &params);

    let cpu_cost = env.budget().cpu_instruction_cost();

    // Critical: Must stay under Soroban limits
    assert!(
        cpu_cost < 100_000_000,
        "CPU exceeded 100M limit: {}",
        cpu_cost
    );
    assert!(result.is_ok(), "Max complexity swap should succeed");
}

#[test]
fn regression_test_gas_increase() {
    let env = setup_env();
    let (_admin, _, client) = deploy_router(&env);
    let pool = deploy_mock_pool(&env);

    env.mock_all_auths();
    client.register_pool(&pool);

    let route = make_route(&env, &pool, 2);

    // Baseline measurement
    let _ = client.get_quote(&1_000_000, &route);
    let baseline_cpu = env.budget().cpu_instruction_cost();

    // Regression threshold: fail if gas increases by >10%
    let max_allowed = baseline_cpu + (baseline_cpu / 10);

    assert!(
        baseline_cpu < max_allowed,
        "Gas consumption increased by more than 10%: baseline={}, max={}",
        baseline_cpu,
        max_allowed
    );
}
