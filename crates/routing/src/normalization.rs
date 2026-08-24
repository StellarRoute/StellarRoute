use crate::error::{Result, RoutingError};

const SCALE_1E7: i128 = 10_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VenueType {
    Sdex,
    Amm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SdexLevelInput {
    pub offer_id: i64,
    pub price: String,
    pub amount: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmmReserveInput {
    pub pool_address: String,
    pub reserve_selling: String,
    pub reserve_buying: String,
    pub fee_bps: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedLiquidityLevel {
    pub venue_type: VenueType,
    pub venue_ref: String,
    pub price_e7: i128,
    pub available_amount_e7: i128,
}

pub fn normalize_sdex_levels(inputs: &[SdexLevelInput]) -> Result<Vec<NormalizedLiquidityLevel>> {
    let mut normalized = Vec::with_capacity(inputs.len());

    for level in inputs {
        if level.offer_id <= 0 {
            return Err(RoutingError::Normalization(
                "offer_id must be positive".to_string(),
            ));
        }

        let price_e7 = parse_decimal_to_e7(&level.price)?;
        let amount_e7 = parse_decimal_to_e7(&level.amount)?;

        if price_e7 <= 0 || amount_e7 <= 0 {
            return Err(RoutingError::InvalidAmount(
                "SDEX price and amount must be > 0".to_string(),
            ));
        }

        normalized.push(NormalizedLiquidityLevel {
            venue_type: VenueType::Sdex,
            venue_ref: level.offer_id.to_string(),
            price_e7,
            available_amount_e7: amount_e7,
        });
    }

    normalized.sort_by_key(|a| a.price_e7);
    Ok(normalized)
}

pub fn normalize_amm_reserve(input: &AmmReserveInput) -> Result<NormalizedLiquidityLevel> {
    if input.pool_address.trim().is_empty() {
        return Err(RoutingError::Normalization(
            "pool_address is required".to_string(),
        ));
    }
    if input.fee_bps > 10_000 {
        return Err(RoutingError::Normalization(
            "fee_bps must be in [0, 10000]".to_string(),
        ));
    }

    let reserve_selling_e7 = parse_decimal_to_e7(&input.reserve_selling)?;
    let reserve_buying_e7 = parse_decimal_to_e7(&input.reserve_buying)?;

    if reserve_selling_e7 <= 0 || reserve_buying_e7 <= 0 {
        return Err(RoutingError::InvalidAmount(
            "AMM reserves must be > 0".to_string(),
        ));
    }

    let fee_numerator = 10_000_i128 - i128::from(input.fee_bps);
    let numerator = reserve_buying_e7
        .checked_mul(fee_numerator)
        .and_then(|v| v.checked_mul(SCALE_1E7))
        .ok_or(RoutingError::Overflow)?;

    let denominator = reserve_selling_e7
        .checked_mul(10_000)
        .ok_or(RoutingError::Overflow)?;

    if denominator == 0 {
        return Err(RoutingError::InvalidAmount(
            "reserve_selling cannot be zero".to_string(),
        ));
    }

    let price_e7 = numerator / denominator;
    if price_e7 <= 0 {
        return Err(RoutingError::InvalidAmount(
            "computed AMM price must be > 0".to_string(),
        ));
    }

    Ok(NormalizedLiquidityLevel {
        venue_type: VenueType::Amm,
        venue_ref: input.pool_address.clone(),
        price_e7,
        available_amount_e7: reserve_selling_e7,
    })
}

pub fn normalize_liquidity(
    sdex_levels: &[SdexLevelInput],
    amm_reserves: &[AmmReserveInput],
) -> Result<Vec<NormalizedLiquidityLevel>> {
    let mut levels = normalize_sdex_levels(sdex_levels)?;
    for reserve in amm_reserves {
        levels.push(normalize_amm_reserve(reserve)?);
    }
    levels.sort_by_key(|a| a.price_e7);
    Ok(levels)
}

fn parse_decimal_to_e7(value: &str) -> Result<i128> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(RoutingError::Normalization(
            "decimal value cannot be empty".to_string(),
        ));
    }

    if trimmed.starts_with('-') {
        return Err(RoutingError::InvalidAmount(
            "negative decimal values are not supported".to_string(),
        ));
    }

    let mut parts = trimmed.split('.');
    let int_part = parts.next().unwrap_or("0");
    let frac_part = parts.next().unwrap_or("");

    if parts.next().is_some() {
        return Err(RoutingError::DecimalPrecision(format!(
            "invalid decimal format: {}",
            value
        )));
    }

    if !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
    {
        return Err(RoutingError::DecimalPrecision(format!(
            "non-digit decimal value: {}",
            value
        )));
    }

    if frac_part.len() > 7 {
        return Err(RoutingError::DecimalPrecision(format!(
            "more than 7 decimal places is not allowed: {}",
            value
        )));
    }

    let int_value = int_part
        .parse::<i128>()
        .map_err(|_| RoutingError::DecimalPrecision(format!("invalid integer part: {}", value)))?;

    let frac_value = if frac_part.is_empty() {
        0
    } else {
        frac_part.parse::<i128>().map_err(|_| {
            RoutingError::DecimalPrecision(format!("invalid fractional part: {}", value))
        })?
    };

    let scale_factor = 10_i128
        .checked_pow((7 - frac_part.len()) as u32)
        .ok_or(RoutingError::Overflow)?;

    int_value
        .checked_mul(SCALE_1E7)
        .and_then(|v| {
            frac_value
                .checked_mul(scale_factor)
                .and_then(|f| v.checked_add(f))
        })
        .ok_or(RoutingError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_sdex_normalization_deterministic_precision() {
        let inputs = vec![SdexLevelInput {
            offer_id: 10,
            price: "1.2345678".to_string(),
            amount: "99.0000001".to_string(),
        }];

        let levels = normalize_sdex_levels(&inputs).unwrap();
        assert_eq!(levels[0].price_e7, 12_345_678);
        assert_eq!(levels[0].available_amount_e7, 990_000_001);
    }

    #[test]
    fn test_rejects_extra_decimal_precision() {
        let input = SdexLevelInput {
            offer_id: 1,
            price: "1.12345678".to_string(),
            amount: "1".to_string(),
        };

        let result = normalize_sdex_levels(&[input]);
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_partial_amm_data() {
        let input = AmmReserveInput {
            pool_address: " ".to_string(),
            reserve_selling: "100".to_string(),
            reserve_buying: "200".to_string(),
            fee_bps: 30,
        };

        let result = normalize_amm_reserve(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_mixed_venues() {
        let sdex = vec![SdexLevelInput {
            offer_id: 7,
            price: "1.2000000".to_string(),
            amount: "10".to_string(),
        }];
        let amm = vec![AmmReserveInput {
            pool_address: "CPOOL123".to_string(),
            reserve_selling: "500".to_string(),
            reserve_buying: "800".to_string(),
            fee_bps: 30,
        }];

        let levels = normalize_liquidity(&sdex, &amm).unwrap();
        assert_eq!(levels.len(), 2);
        assert!(levels[0].price_e7 <= levels[1].price_e7);
    }

    #[test]
    fn benchmark_normalization_transform_latency() {
        let mut sdex = Vec::new();
        for i in 1..=1000 {
            sdex.push(SdexLevelInput {
                offer_id: i,
                price: "1.0100000".to_string(),
                amount: "100.0000000".to_string(),
            });
        }

        let amm = vec![AmmReserveInput {
            pool_address: "CPOOLBENCH".to_string(),
            reserve_selling: "250000.0000000".to_string(),
            reserve_buying: "300000.0000000".to_string(),
            fee_bps: 30,
        }];

        let start = Instant::now();
        let levels = normalize_liquidity(&sdex, &amm).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(levels.len(), 1001);
        assert!(
            elapsed.as_millis() < 200,
            "normalization too slow: {elapsed:?}"
        );
    }
}
