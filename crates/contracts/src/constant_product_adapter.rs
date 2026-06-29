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

        numerator / denominator
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
            let reserve_in = e.storage().instance().get(&symbol_short!("RIN")).unwrap_or(0);
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
    fn setup(env: &Env, reserve_in: i128, reserve_out: i128) -> PoolAdapterClient {
        let pool_id = env.register_contract(None, MockPool);
        MockPoolClient::new(env, &pool_id).set_rsrvs(&reserve_in, &reserve_out);

        let adapter_id = env.register_contract(None, ConstantProductAdapter);
        env.as_contract(&adapter_id, || {
            env.storage().instance().set(&symbol_short!("POOL"), &pool_id);
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
}
