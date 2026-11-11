use stellarroute_routing::impact::AmmQuoteCalculator;

#[test]
fn test_amm_constant_product_reference_vectors() {
    let calc = AmmQuoteCalculator;

    // Case 1: Standard 0.3% fee
    // x=1000, y=1000, in=100, fee=30bps
    // in_with_fee = 100 * 9970 / 10000 = 99
    // out = (99 * 1000) / (1000 + 99) = 99000 / 1099 = 90.081... -> 90
    let (output, impact) = calc.quote_constant_product(100, 1000, 1000, 30).unwrap();
    assert_eq!(output, 90);
    assert!(impact > 0);

    // Case 2: Zero fee
    // x=1000, y=1000, in=100, fee=0
    // out = (100 * 1000) / (1000 + 100) = 100000 / 1100 = 90.909... -> 90
    let (output, _) = calc.quote_constant_product(100, 1000, 1000, 0).unwrap();
    assert_eq!(output, 90);

    // Case 3: Large reserves (low impact)
    // x=10^12, y=10^12, in=10^6, fee=30
    // in_with_fee = 997,000
    // out = (997,000 * 10^12) / (10^12 + 997,000) = 996,999.005... -> 996,999
    let (output, impact) = calc
        .quote_constant_product(1_000_000, 1_000_000_000_000, 1_000_000_000_000, 30)
        .unwrap();
    assert_eq!(output, 996_999);
    assert_eq!(impact, 30); // Impact includes the 30bps fee in this implementation

    // Case 4: High fee (1%)
    // x=1000, y=1000, in=100, fee=100
    // in_with_fee = 99
    // out = 90 (matches case 1 due to integer rounding in this small example)
    let (output, _) = calc.quote_constant_product(100, 1000, 1000, 100).unwrap();
    assert_eq!(output, 90);
}

#[test]
fn test_amm_impact_scaling() {
    let calc = AmmQuoteCalculator;
    let x = 10_000_000; // 1.0 in e7
    let y = 10_000_000;

    // Trade 1% of reserve
    let (out1, impact1) = calc.quote_constant_product(100_000, x, y, 0).unwrap();
    // Trade 10% of reserve
    let (out2, impact2) = calc.quote_constant_product(1_000_000, x, y, 0).unwrap();

    assert!(impact2 > impact1);
    assert!(out2 < out1 * 10); // More slippage on larger trade
}

#[test]
fn test_amm_reverse_quote_conformance() {
    let calc = AmmQuoteCalculator;
    let x = 1_000_000;
    let y = 1_000_000;
    let fee = 30;
    let amount_in = 10_000;

    let (amount_out, _) = calc.quote_constant_product(amount_in, x, y, fee).unwrap();
    let (recovered_in, _) = calc
        .quote_constant_product_reverse(amount_out, x, y, fee)
        .unwrap();

    // recovered_in should be close to amount_in (usually slightly larger due to rounding up)
    assert!(recovered_in >= amount_in);
    assert!(recovered_in < amount_in + 5);
}

#[test]
fn test_amm_price_impact_consistent_across_pool_prices() {
    let calc = AmmQuoteCalculator;

    // 0.1% of reserve_in swap with 30 bps fee across price 4, price 0.25, and price 1 pools
    let (_, impact_price_4) = calc
        .quote_constant_product(10_000_000, 10_000_000_000, 40_000_000_000, 30)
        .unwrap();
    let (_, impact_price_0_25_same_fraction) = calc
        .quote_constant_product(40_000_000, 40_000_000_000, 10_000_000_000, 30)
        .unwrap();
    let (_, impact_price_1) = calc
        .quote_constant_product(10_000_000, 10_000_000_000, 10_000_000_000, 30)
        .unwrap();

    // Exact shortfall is ~39.93 bps (30 bps fee + ~9.96 bps curve), floored to 39 bps
    assert_eq!(impact_price_4, 39);
    assert_eq!(impact_price_0_25_same_fraction, 39);
    assert_eq!(impact_price_1, 39);

    // Fixed nominal 1e7 input into 4e10 / 1e10 (0.025% of reserve_in): ~32 bps instead of 150_521 bps
    let (_, impact_price_0_25_nominal) = calc
        .quote_constant_product(10_000_000, 40_000_000_000, 10_000_000_000, 30)
        .unwrap();
    assert_eq!(impact_price_0_25_nominal, 32);

    // Reverse quote on non-1:1 pools also uses consistent units
    let (out_4, fwd_impact_4) = calc
        .quote_constant_product(10_000_000, 10_000_000_000, 40_000_000_000, 30)
        .unwrap();
    let (_, rev_impact_4) = calc
        .quote_constant_product_reverse(out_4, 10_000_000_000, 40_000_000_000, 30)
        .unwrap();
    assert!(rev_impact_4.abs_diff(fwd_impact_4) <= 1);

    let (out_quarter, fwd_impact_quarter) = calc
        .quote_constant_product(40_000_000, 40_000_000_000, 10_000_000_000, 30)
        .unwrap();
    let (_, rev_impact_quarter) = calc
        .quote_constant_product_reverse(out_quarter, 40_000_000_000, 10_000_000_000, 30)
        .unwrap();
    assert!(rev_impact_quarter.abs_diff(fwd_impact_quarter) <= 1);
}

#[test]
fn test_amm_overflow_and_impact_cap() {
    use stellarroute_routing::error::RoutingError;
    use stellarroute_routing::impact::OrderbookImpactCalculator;

    let calc = AmmQuoteCalculator;

    // Checked multiplication overflow in fee application or reserve product returns Overflow
    assert!(matches!(
        calc.quote_constant_product(i128::MAX, 1_000_000_000, 1_000_000_000, 30),
        Err(RoutingError::Overflow)
    ));
    assert!(matches!(
        calc.quote_constant_product(1_000_000_000, 1_000_000_000, i128::MAX, 30),
        Err(RoutingError::Overflow)
    ));
    assert!(matches!(
        calc.quote_constant_product_reverse(i128::MAX - 1, i128::MAX, i128::MAX, 30),
        Err(RoutingError::Overflow)
    ));

    let ob_calc = OrderbookImpactCalculator;
    assert!(matches!(
        ob_calc.calculate_impact(1_000_000_000, &[(i128::MAX, 1_000_000_000)]),
        Err(RoutingError::Overflow)
    ));

    // Extreme trade draining nearly the entire pool never exceeds 10_000 bps
    let (_, extreme_impact) = calc
        .quote_constant_product(1_000_000_000_000_000, 1_000_000, 1_000_000_000, 9_999)
        .unwrap();
    assert!(extreme_impact <= 10_000);
}

mod proptests {
    use proptest::prelude::*;
    use stellarroute_routing::impact::AmmQuoteCalculator;

    proptest! {
        #[test]
        fn prop_impact_grows_with_trade_size_and_stays_bounded(
            reserve_in in 10_000_000_000i128..=1_000_000_000_000_000i128,
            reserve_out in 10_000_000_000i128..=1_000_000_000_000_000i128,
            small_bps in 10i128..=1_000i128,
            extra_bps in 10i128..=3_000i128,
            fee_bps in 0u32..=1_000u32,
        ) {
            let calc = AmmQuoteCalculator;
            let amount_small = (reserve_in / 10_000) * small_bps;
            let amount_large = (reserve_in / 10_000) * (small_bps + extra_bps);

            let (_, impact_small) = calc
                .quote_constant_product(amount_small, reserve_in, reserve_out, fee_bps)
                .unwrap();
            let (_, impact_large) = calc
                .quote_constant_product(amount_large, reserve_in, reserve_out, fee_bps)
                .unwrap();

            prop_assert!(impact_small <= 10_000);
            prop_assert!(impact_large <= 10_000);
            prop_assert!(
                impact_large >= impact_small,
                "expected impact_large ({impact_large}) >= impact_small ({impact_small})"
            );
        }

        #[test]
        fn prop_impact_is_pool_side_symmetric(
            reserve_a in 10_000_000_000i128..=1_000_000_000_000_000i128,
            reserve_b in 10_000_000_000i128..=1_000_000_000_000_000i128,
            trade_fraction_ppm in 100i128..=250_000i128,
            fee_bps in 0u32..=1_000u32,
        ) {
            let calc = AmmQuoteCalculator;
            // Quote identical pool fraction on A->B and B->A (exact multiples of 10_000 avoid fee-rounding asymmetry)
            let unit_a = (reserve_a / 1_000_000 / 10_000).max(1) * 10_000;
            let unit_b = (reserve_b / 1_000_000 / 10_000).max(1) * 10_000;
            let norm_reserve_a = unit_a * 1_000_000;
            let norm_reserve_b = unit_b * 1_000_000;
            let amount_a = unit_a * trade_fraction_ppm;
            let amount_b = unit_b * trade_fraction_ppm;

            let (_, impact_ab) = calc
                .quote_constant_product(amount_a, norm_reserve_a, norm_reserve_b, fee_bps)
                .unwrap();
            let (_, impact_ba) = calc
                .quote_constant_product(amount_b, norm_reserve_b, norm_reserve_a, fee_bps)
                .unwrap();

            prop_assert!(impact_ab <= 10_000);
            prop_assert!(impact_ba <= 10_000);
            prop_assert!(
                impact_ab.abs_diff(impact_ba) <= 1,
                "expected side symmetry within 1 bps, got A->B={impact_ab} vs B->A={impact_ba}"
            );
        }
    }
}
