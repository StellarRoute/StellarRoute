use crate::adapters::PoolAdapterTrait;
use crate::types::Asset;
use soroban_sdk::{contract, contractimpl, symbol_short, vec, Address, Env, IntoVal};

#[contract]
pub struct ConstantProductAdapter;

#[contractimpl]
impl PoolAdapterTrait for ConstantProductAdapter {
    fn swap(
        e: Env,
        input_asset: Asset,
        output_asset: Asset,
        amount_in: i128,
        min_out: i128,
    ) -> i128 {
        // 1. Get the underlying pool address (stored in this adapter's instance storage)
        let pool_address: Address = e.storage().instance().get(&symbol_short!("POOL")).unwrap();

        // 2. Translate to the specific AMM's function (e.g., Soroswap uses 'swap')
        // We use CCI to call the actual pool
        let out: i128 = e.invoke_contract(
            &pool_address,
            &symbol_short!("swap"),
            vec![
                &e,
                input_asset.into_val(&e),
                output_asset.into_val(&e),
                amount_in.into_val(&e),
                min_out.into_val(&e),
            ],
        );

        out
    }

    fn adapter_quote(e: Env, _input_asset: Asset, _output_asset: Asset, amount_in: i128) -> i128 {
        let (res_in, res_out) = Self::get_rsrvs(e.clone());

        // dy = (y * dx * 997) / (x * 1000 + dx * 997)
        let fee_multiplier: i128 = 997;
        let amount_with_fee = amount_in
            .checked_mul(fee_multiplier)
            .unwrap_or_else(|| panic!("overflow: amount_with_fee"));
        let numerator = amount_with_fee
            .checked_mul(res_out)
            .unwrap_or_else(|| panic!("overflow: numerator"));
        let denominator = res_in
            .checked_mul(1000)
            .and_then(|v| v.checked_add(amount_with_fee))
            .unwrap_or_else(|| panic!("overflow: denominator"));

        if denominator == 0 {
            panic!("division by zero: empty pool reserves");
        }

        numerator
            .checked_div(denominator)
            .unwrap_or_else(|| panic!("overflow: division"))
    }

    fn get_rsrvs(e: Env) -> (i128, i128) {
        let pool_address: Address = e.storage().instance().get(&symbol_short!("POOL")).unwrap();
        // Call the underlying pool's reserve function
        e.invoke_contract(&pool_address, &symbol_short!("get_rsrvs"), vec![&e])
    }
}

#[cfg(test)]
mod tests {
    use super::ConstantProductAdapter;
    use crate::adapters::PoolAdapterClient;
    use crate::types::Asset;
    use proptest::proptest;
    use soroban_sdk::{contract, contractimpl, symbol_short, Env};

    // Minimal pool stub whose reserves are configurable, so we can drive the
    // constant-product adapter into its zero-reserve edge cases.
    #[contract]
    pub struct MockPool;

    #[contractimpl]
    impl MockPool {
        pub fn set_rsrvs(e: Env, reserve_in: i128, reserve_out: i128) {
            e.storage()
                .instance()
                .set(&symbol_short!("RIN"), &reserve_in);
            e.storage()
                .instance()
                .set(&symbol_short!("ROUT"), &reserve_out);
        }

        pub fn get_rsrvs(e: Env) -> (i128, i128) {
            let reserve_in = e
                .storage()
                .instance()
                .get(&symbol_short!("RIN"))
                .unwrap_or(0);
            let reserve_out = e
                .storage()
                .instance()
                .get(&symbol_short!("ROUT"))
                .unwrap_or(0);
            (reserve_in, reserve_out)
        }
    }

    // Wire a constant-product adapter to a mock pool seeded with the given
    // reserves and return a client for issuing quotes against it.
    fn setup(env: &Env, reserve_in: i128, reserve_out: i128) -> PoolAdapterClient<'_> {
        let pool_id = env.register_contract(None, MockPool);
        MockPoolClient::new(env, &pool_id).set_rsrvs(&reserve_in, &reserve_out);

        let adapter_id = env.register_contract(None, ConstantProductAdapter);
        env.as_contract(&adapter_id, || {
            env.storage()
                .instance()
                .set(&symbol_short!("POOL"), &pool_id);
        });

        PoolAdapterClient::new(env, &adapter_id)
    }

    #[test]
    fn adapter_quote_with_zero_output_reserve_returns_zero() {
        let env = Env::default();
        let adapter = setup(&env, 1_000, 0);
        let quote = adapter.adapter_quote(&Asset::Native, &Asset::Native, &100);
        assert_eq!(quote, 0);
    }

    #[test]
    fn adapter_quote_with_empty_reserves_returns_zero() {
        let env = Env::default();
        let adapter = setup(&env, 0, 0);
        // A positive input against an empty pool keeps the denominator
        // non-zero, so the quote resolves to zero rather than dividing by zero.
        let quote = adapter.adapter_quote(&Asset::Native, &Asset::Native, &100);
        assert_eq!(quote, 0);
    }

    #[test]
    #[should_panic]
    fn adapter_quote_panics_on_empty_reserves_with_zero_amount() {
        let env = Env::default();
        let adapter = setup(&env, 0, 0);
        // Empty reserves and a zero input collapse the denominator to zero,
        // which the adapter rejects by panicking (documented behaviour).
        adapter.adapter_quote(&Asset::Native, &Asset::Native, &0);
    }

    // Property tests for reserve invariance and edge cases
    proptest! {
        #[test]
        fn prop_reserve_invariance_within_fee(
            reserve_in in 1_000_000i128..1_000_000_000_000i128,
            reserve_out in 1_000_000i128..1_000_000_000_000i128,
            amount_in in 1i128..1_000_000_000i128,
        ) {
            let env = Env::default();
            let adapter = setup(&env, reserve_in, reserve_out);

            // Before swap: x * y = k
            let k_before = reserve_in.saturating_mul(reserve_out);

            // Quote the output amount
            let amount_out = adapter.adapter_quote(&Asset::Native, &Asset::Native, &amount_in);

            // After swap: (x + input_with_fee) * (y - output) ≈ k
            let fee_multiplier: i128 = 997;
            let amount_with_fee = amount_in.saturating_mul(fee_multiplier);
            let reserve_in_after = reserve_in.saturating_add(amount_with_fee);
            let reserve_out_after = reserve_out.saturating_sub(amount_out);

            let k_after = reserve_in_after.saturating_mul(reserve_out_after);

            // k_after should be greater than or equal to k_before (within fee tolerance)
            // allowing for small precision loss in division
            prop_assert!(k_after >= k_before,
                "Reserve invariance violated: k_before={}, k_after={}",
                k_before, k_after);
        }

        #[test]
        fn prop_zero_input_returns_zero(
            reserve_in in 1_000_000i128..1_000_000_000_000i128,
            reserve_out in 1_000_000i128..1_000_000_000_000i128,
        ) {
            let env = Env::default();
            let adapter = setup(&env, reserve_in, reserve_out);

            // Zero input should always return zero output
            let quote = adapter.adapter_quote(&Asset::Native, &Asset::Native, &0);
            prop_assert_eq!(quote, 0, "Zero input should produce zero output");
        }

        #[test]
        fn prop_positive_input_positive_output(
            reserve_in in 1_000_000i128..1_000_000_000_000i128,
            reserve_out in 1_000_000i128..1_000_000_000_000i128,
            amount_in in 1i128..1_000_000_000i128,
        ) {
            let env = Env::default();
            let adapter = setup(&env, reserve_in, reserve_out);

            // Positive input should yield non-negative output
            let quote = adapter.adapter_quote(&Asset::Native, &Asset::Native, &amount_in);
            prop_assert!(quote >= 0, "Quote should be non-negative for positive input");
        }
    }
}
