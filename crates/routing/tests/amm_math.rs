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
