//! Price impact and execution calculators for SDEX and AMM

use crate::error::{Result, RoutingError};

const SCALE_1E7: i128 = 10_000_000;
const BASIS_POINTS: i128 = 10_000;

/// Type alias for partial fill data: (fill_amount, price)
pub type PartialFill = (i128, i128);

/// Type alias for impact calculation result: (impact_bps, effective_price, partial_fills)
pub type ImpactResult = (u32, i128, Vec<PartialFill>);

/// Orderbook depth impact calculator
pub struct OrderbookImpactCalculator;

impl OrderbookImpactCalculator {
    /// Calculate execution impact based on orderbook depth consumption
    /// Returns (impact_bps, effective_price, partial_fills)
    pub fn calculate_impact(
        &self,
        amount_in: i128,
        orderbook_levels: &[(i128, i128)], // (price_e7, available_amount_e7)
    ) -> Result<ImpactResult> {
        if amount_in <= 0 {
            return Err(RoutingError::InvalidAmount(
                "amount_in must be positive".to_string(),
            ));
        }

        if orderbook_levels.is_empty() {
            return Err(RoutingError::InsufficientLiquidity(
                "empty orderbook".to_string(),
            ));
        }

        let mut remaining = amount_in;
        let mut total_cost = 0i128;
        let mut partial_fills = Vec::new();

        // Process each level
        for (price_e7, available_e7) in orderbook_levels {
            if *price_e7 <= 0 || *available_e7 <= 0 {
                return Err(RoutingError::InvalidAmount("invalid level".to_string()));
            }

            if remaining == 0 {
                break;
            }

            let fill_amount = remaining.min(*available_e7);
            let cost = fill_amount
                .checked_mul(*price_e7)
                .ok_or(RoutingError::Overflow)?
                .checked_div(SCALE_1E7)
                .ok_or(RoutingError::Overflow)?;

            total_cost = total_cost.checked_add(cost).ok_or(RoutingError::Overflow)?;
            partial_fills.push((fill_amount, *price_e7));

            remaining = remaining
                .checked_sub(fill_amount)
                .ok_or(RoutingError::Overflow)?;
        }

        if remaining > 0 {
            return Err(RoutingError::InsufficientLiquidity(format!(
                "insufficient liquidity: {} remaining",
                remaining
            )));
        }

        // Effective price
        let effective_price = total_cost
            .checked_mul(SCALE_1E7)
            .ok_or(RoutingError::Overflow)?
            .checked_div(amount_in)
            .ok_or(RoutingError::Overflow)?;

        // VWAP price (first level as baseline)
        let baseline_price = orderbook_levels[0].0;

        // Impact in bps
        let impact_bps = if effective_price > baseline_price {
            let diff = effective_price
                .checked_sub(baseline_price)
                .ok_or(RoutingError::Overflow)?;
            let raw = diff
                .checked_mul(BASIS_POINTS)
                .ok_or(RoutingError::Overflow)?
                .checked_div(baseline_price)
                .ok_or(RoutingError::Overflow)?;
            u32::try_from(raw).map_err(|_| RoutingError::Overflow)?
        } else {
            0
        };

        Ok((impact_bps, effective_price, partial_fills))
    }

    /// Handle thin and empty orderbooks
    pub fn is_book_valid(&self, orderbook_levels: &[(i128, i128)]) -> bool {
        !orderbook_levels.is_empty() && orderbook_levels.iter().all(|(p, a)| p > &0 && a > &0)
    }
}

/// AMM constant-product price quote calculator
pub struct AmmQuoteCalculator;

impl AmmQuoteCalculator {
    /// Calculate swap output using constant-product formula: x * y = k
    /// Returns (output_amount, price_impact_bps)
    pub fn quote_constant_product(
        &self,
        amount_in: i128,
        reserve_in: i128,
        reserve_out: i128,
        fee_bps: u32,
    ) -> Result<(i128, u32)> {
        if amount_in <= 0 {
            return Err(RoutingError::InvalidAmount(
                "amount_in must be positive".to_string(),
            ));
        }

        if reserve_in <= 0 || reserve_out <= 0 {
            return Err(RoutingError::InvalidAmount(
                "reserves must be positive".to_string(),
            ));
        }

        if fee_bps > 10_000 {
            return Err(RoutingError::InvalidAmount(
                "fee_bps must be in [0, 10000]".to_string(),
            ));
        }

        // Apply fee: amount_in_with_fee = amount_in * (1 - fee_bps / 10000)
        let fee_multiplier = BASIS_POINTS
            .checked_sub(i128::from(fee_bps))
            .ok_or(RoutingError::Overflow)?;
        let amount_in_with_fee = amount_in
            .checked_mul(fee_multiplier)
            .ok_or(RoutingError::Overflow)?
            .checked_div(BASIS_POINTS)
            .ok_or(RoutingError::Overflow)?;

        // Constant product: x * y = k
        // output = (amount_in_with_fee * reserve_out) / (reserve_in + amount_in_with_fee)
        let numerator = amount_in_with_fee
            .checked_mul(reserve_out)
            .ok_or(RoutingError::Overflow)?;

        let denominator = reserve_in
            .checked_add(amount_in_with_fee)
            .ok_or(RoutingError::Overflow)?;

        if denominator == 0 {
            return Err(RoutingError::InvalidAmount(
                "invalid reserve state".to_string(),
            ));
        }

        let output = numerator
            .checked_div(denominator)
            .ok_or(RoutingError::Overflow)?;

        if output <= 0 {
            return Err(RoutingError::InsufficientLiquidity(
                "zero output from AMM".to_string(),
            ));
        }

        let impact_bps =
            Self::compute_price_impact_bps(amount_in, output, reserve_in, reserve_out)?;

        Ok((output, impact_bps))
    }

    /// Calculate swap input needed for exact output (reverse quote)
    pub fn quote_constant_product_reverse(
        &self,
        amount_out: i128,
        reserve_in: i128,
        reserve_out: i128,
        fee_bps: u32,
    ) -> Result<(i128, u32)> {
        if amount_out <= 0 || amount_out >= reserve_out {
            return Err(RoutingError::InvalidAmount(
                "invalid amount_out".to_string(),
            ));
        }

        if reserve_in <= 0 || reserve_out <= 0 {
            return Err(RoutingError::InvalidAmount(
                "reserves must be positive".to_string(),
            ));
        }

        // Reverse calculation: input_required = (reserve_in * amount_out) / (reserve_out - amount_out) / (1 - fee)
        let numerator = reserve_in
            .checked_mul(amount_out)
            .ok_or(RoutingError::Overflow)?;

        let denominator = reserve_out
            .checked_sub(amount_out)
            .ok_or(RoutingError::Overflow)?;

        if denominator <= 0 {
            return Err(RoutingError::InsufficientLiquidity(
                "insufficient reserve_out".to_string(),
            ));
        }

        let input_before_fee = numerator
            .checked_div(denominator)
            .ok_or(RoutingError::Overflow)?
            .checked_add(1) // Round up
            .ok_or(RoutingError::Overflow)?;

        // Account for fee
        let fee_divisor = BASIS_POINTS
            .checked_sub(i128::from(fee_bps))
            .ok_or(RoutingError::Overflow)?;
        if fee_divisor <= 0 {
            return Err(RoutingError::InvalidAmount("invalid fee".to_string()));
        }

        let input_required = input_before_fee
            .checked_mul(BASIS_POINTS)
            .ok_or(RoutingError::Overflow)?
            .checked_div(fee_divisor)
            .ok_or(RoutingError::Overflow)?
            .checked_add(1) // Round up
            .ok_or(RoutingError::Overflow)?;

        let impact_bps =
            Self::compute_price_impact_bps(input_required, amount_out, reserve_in, reserve_out)?;

        Ok((input_required, impact_bps))
    }

    /// Compute price impact in basis points using consistent output-per-input units:
    /// `(spot_out_per_in - exec_out_per_in) / spot_out_per_in`, clamped to `[0, 10_000]`.
    fn compute_price_impact_bps(
        amount_in: i128,
        amount_out: i128,
        reserve_in: i128,
        reserve_out: i128,
    ) -> Result<u32> {
        let spot_notional = amount_in
            .checked_mul(reserve_out)
            .ok_or(RoutingError::Overflow)?;
        let exec_notional = amount_out
            .checked_mul(reserve_in)
            .ok_or(RoutingError::Overflow)?;

        if spot_notional <= 0 {
            return Err(RoutingError::InvalidAmount(
                "invalid spot notional".to_string(),
            ));
        }

        if exec_notional >= spot_notional {
            return Ok(0);
        }

        let shortfall = spot_notional
            .checked_sub(exec_notional)
            .ok_or(RoutingError::Overflow)?;
        let raw_bps = shortfall
            .checked_mul(BASIS_POINTS)
            .ok_or(RoutingError::Overflow)?
            .checked_div(spot_notional)
            .ok_or(RoutingError::Overflow)?;

        Ok(raw_bps.clamp(0, BASIS_POINTS) as u32)
    }

    /// Detect edge cases: large/small trades
    pub fn validate_trade_size(
        &self,
        amount_in: i128,
        reserve_in: i128,
        _max_impact_bps: u32,
    ) -> Result<()> {
        // Trade size should not exceed 50% of reserve (avoid 50% slippage)
        let max_safe_amount = reserve_in / 2;
        if amount_in > max_safe_amount {
            return Err(RoutingError::InvalidAmount(
                "trade size exceeds 50% of reserve".to_string(),
            ));
        }

        if amount_in < 1_000_000 {
            // Less than 0.1 in e7 precision
            return Err(RoutingError::InvalidAmount(
                "trade size too small".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_partial_fills() {
        let calc = OrderbookImpactCalculator;
        let levels = vec![
            (SCALE_1E7, 1_000_000_000),
            (SCALE_1E7 * 11 / 10, 500_000_000),
        ];

        let result = calc.calculate_impact(1_500_000_000, &levels);
        assert!(result.is_ok());

        let (impact_bps, _effective_price, fills) = result.unwrap();
        assert_eq!(fills.len(), 2);
        assert!(impact_bps > 0);
    }

    #[test]
    fn test_amm_constant_product() {
        let calc = AmmQuoteCalculator;
        let result = calc.quote_constant_product(
            1_000_000_000, // 100 in e7
            10_000_000_000,
            10_000_000_000,
            30, // 0.3% fee
        );

        assert!(result.is_ok());
        let (output, _impact) = result.unwrap();
        assert!(output > 0);
        assert!(output < 1_000_000_000); // Some slippage
    }

    #[test]
    fn test_amm_trade_size_validation() {
        let calc = AmmQuoteCalculator;
        assert!(calc
            .validate_trade_size(6_000_000_000, 10_000_000_000, 500)
            .is_err());
        assert!(calc
            .validate_trade_size(3_000_000_000, 10_000_000_000, 500)
            .is_ok());
    }

    #[test]
    fn test_empty_orderbook() {
        let calc = OrderbookImpactCalculator;
        let result = calc.calculate_impact(1_000_000_000, &[]);
        assert!(result.is_err());
    }
}
