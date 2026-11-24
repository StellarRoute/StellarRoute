//! Exact decimal-string to Stellar stroops (`10^-7` unit) conversion.
//!
//! Avoids IEEE-754 `f64` precision loss above `2^53` stroops (~900M units),
//! double-rounding drift between `2^51` and `2^53` stroops, silent rounding of
//! >7 fractional digits, and `i64::MAX as f64` (`2^63`) cast saturation.

use thiserror::Error;

/// Number of stroops per 1.0 Stellar asset unit (`10^7`).
pub const STROOPS_PER_UNIT: i64 = 10_000_000;

const STROOPS_PER_UNIT_U128: u128 = 10_000_000;
const MAX_STROOPS_U128: u128 = i64::MAX as u128;
const MAX_WHOLE_UNITS_U128: u128 = MAX_STROOPS_U128 / STROOPS_PER_UNIT_U128;

/// Validation error returned when parsing a Stellar decimal amount into stroops.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AmountError {
    #[error("amount must not be empty")]
    Empty,
    #[error("amount must be greater than zero")]
    Negative,
    #[error("amount must be greater than zero")]
    NonPositive,
    #[error("scientific notation is not allowed")]
    ExponentNotAllowed,
    #[error("amount must have at most 7 decimal places")]
    TooManyDecimals,
    #[error("amount exceeds maximum representable stroops (i64::MAX)")]
    Overflow,
    #[error("amount must be a canonical base-10 number")]
    InvalidFormat,
}

/// Parse a base-10 Stellar amount string directly into exact `i64` stroops.
///
/// Rejects:
/// - empty strings, whitespace, or `+` prefixes,
/// - negative numbers (`-`),
/// - zero (`"0"`, `"0.0000000"`),
/// - scientific notation (`"1e3"`, `"1E-7"`),
/// - non-numeric tokens (`"NaN"`, `"inf"`, `"Infinity"`),
/// - bare leading/trailing decimal points (`".5"`, `"5."`) or multiple dots,
/// - more than 7 fractional digits (`"0.00000001"`),
/// - values above `i64::MAX` stroops (`922_337_203_685.4775807`).
pub fn parse_stroops(s: &str) -> Result<i64, AmountError> {
    if s.is_empty() {
        return Err(AmountError::Empty);
    }

    if s.starts_with('-') {
        return Err(AmountError::Negative);
    }

    if s.bytes().any(|b| b == b'e' || b == b'E')
        && !s.eq_ignore_ascii_case("inf")
        && !s.eq_ignore_ascii_case("infinity")
    {
        return Err(AmountError::ExponentNotAllowed);
    }

    let (whole_str, frac_opt) = match s.split_once('.') {
        Some((w, f)) => {
            if f.contains('.') {
                return Err(AmountError::InvalidFormat);
            }
            (w, Some(f))
        }
        None => (s, None),
    };

    if whole_str.is_empty() || !whole_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AmountError::InvalidFormat);
    }

    let mut frac_stroops: u128 = 0;
    if let Some(frac_str) = frac_opt {
        if frac_str.is_empty() || !frac_str.bytes().all(|b| b.is_ascii_digit()) {
            return Err(AmountError::InvalidFormat);
        }
        if frac_str.len() > 7 {
            return Err(AmountError::TooManyDecimals);
        }
        for b in frac_str.bytes() {
            frac_stroops = frac_stroops * 10 + u128::from(b - b'0');
        }
        for _ in frac_str.len()..7 {
            frac_stroops *= 10;
        }
    }

    let mut whole_units: u128 = 0;
    for b in whole_str.bytes() {
        whole_units = whole_units
            .checked_mul(10)
            .and_then(|v| v.checked_add(u128::from(b - b'0')))
            .ok_or(AmountError::Overflow)?;
        if whole_units > MAX_WHOLE_UNITS_U128 {
            return Err(AmountError::Overflow);
        }
    }

    let total_stroops = whole_units
        .checked_mul(STROOPS_PER_UNIT_U128)
        .and_then(|v| v.checked_add(frac_stroops))
        .ok_or(AmountError::Overflow)?;

    if total_stroops == 0 {
        return Err(AmountError::NonPositive);
    }
    if total_stroops > MAX_STROOPS_U128 {
        return Err(AmountError::Overflow);
    }

    Ok(total_stroops as i64)
}

/// Format positive `i64` stroops as a canonical 7-decimal Stellar amount string.
pub fn format_stroops(stroops: i64) -> String {
    debug_assert!(stroops >= 0, "stroops must be non-negative");
    let whole = stroops / STROOPS_PER_UNIT;
    let frac = (stroops % STROOPS_PER_UNIT).unsigned_abs();
    format!("{whole}.{frac:07}")
}

/// Convert validated `i64` stroops to `f64` units strictly for read-only quote pricing.
pub fn stroops_to_f64_lossy(stroops: i64) -> f64 {
    (stroops as f64) / (STROOPS_PER_UNIT as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn legacy_f64_stroops(s: &str) -> i64 {
        let amount: f64 = s.parse().expect("valid f64");
        (amount * 10_000_000.0).round() as i64
    }

    fn format_with_trimmed_zeros(stroops: i64, min_decimals: usize) -> String {
        let full = format_stroops(stroops);
        let (whole, frac) = full.split_once('.').unwrap();
        let trimmed_frac = frac.trim_end_matches('0');
        let keep = trimmed_frac.len().max(min_decimals.min(7));
        if keep == 0 {
            whole.to_string()
        } else {
            format!("{whole}.{}", &frac[..keep])
        }
    }

    #[test]
    fn parses_standard_and_boundary_amounts_exactly() {
        assert_eq!(parse_stroops("1"), Ok(10_000_000));
        assert_eq!(parse_stroops("100"), Ok(1_000_000_000));
        assert_eq!(parse_stroops("99.5000000"), Ok(995_000_000));
        assert_eq!(parse_stroops("0.0000001"), Ok(1));
        assert_eq!(parse_stroops("00012.3400"), Ok(123_400_000));

        // Exact above 2^53 stroops where f64 loses 1 stroop
        assert_eq!(
            parse_stroops("900719925.4740993"),
            Ok(9_007_199_254_740_993)
        );

        // Regression for IEEE-754 double-rounding drift below 2^53 (around 2^52 stroops):
        // `"450359962.7320520"` is 4_503_599_627_320_520 stroops, whereas
        // `(f64 * 1e7).round()` produced 4_503_599_627_320_521.
        assert_eq!(
            parse_stroops("450359962.7320520"),
            Ok(4_503_599_627_320_520)
        );

        // Exact i64::MAX stroops
        assert_eq!(parse_stroops("922337203685.4775807"), Ok(i64::MAX));
    }

    #[test]
    fn rejects_invalid_and_out_of_range_inputs() {
        assert_eq!(parse_stroops(""), Err(AmountError::Empty));
        assert_eq!(parse_stroops("0"), Err(AmountError::NonPositive));
        assert_eq!(parse_stroops("0.0000000"), Err(AmountError::NonPositive));
        assert_eq!(parse_stroops("-1"), Err(AmountError::Negative));
        assert_eq!(parse_stroops("-0.0000001"), Err(AmountError::Negative));
        assert_eq!(parse_stroops("1e3"), Err(AmountError::ExponentNotAllowed));
        assert_eq!(parse_stroops("1E-7"), Err(AmountError::ExponentNotAllowed));
        assert_eq!(parse_stroops("NaN"), Err(AmountError::InvalidFormat));
        assert_eq!(parse_stroops("inf"), Err(AmountError::InvalidFormat));
        assert_eq!(parse_stroops("Infinity"), Err(AmountError::InvalidFormat));
        assert_eq!(parse_stroops("-inf"), Err(AmountError::Negative));
        assert_eq!(parse_stroops("+10"), Err(AmountError::InvalidFormat));
        assert_eq!(parse_stroops(" 10"), Err(AmountError::InvalidFormat));
        assert_eq!(parse_stroops(".5"), Err(AmountError::InvalidFormat));
        assert_eq!(parse_stroops("5."), Err(AmountError::InvalidFormat));
        assert_eq!(parse_stroops("1.2.3"), Err(AmountError::InvalidFormat));
        assert_eq!(
            parse_stroops("0.00000001"),
            Err(AmountError::TooManyDecimals)
        );
        assert_eq!(
            parse_stroops("1.00000000"),
            Err(AmountError::TooManyDecimals)
        );

        // i64::MAX + 1 stroops (2^63, which previously passed `scaled > i64::MAX as f64` and saturated)
        assert_eq!(
            parse_stroops("922337203685.4775808"),
            Err(AmountError::Overflow)
        );
        assert_eq!(
            parse_stroops("999999999999999999999999999999"),
            Err(AmountError::Overflow)
        );
    }

    proptest! {
        /// Below `10^7 * 2^28` (`2_684_354_560_000_000` stroops, ~268.4M units, > 2^51 stroops),
        /// combined IEEE-754 decimal-parse + `* 1e7` rounding error is strictly `< 0.5` stroops,
        /// so `parse_stroops` is 100% bit-for-bit identical to the legacy `f64` path.
        #[test]
        fn prop_matches_legacy_f64_exactly_for_normal_amounts(
            stroops in 1i64..=2_684_354_560_000_000i64,
            min_decimals in 0usize..=7usize,
            leading_zeros in 0usize..=3usize,
        ) {
            let formatted = format!(
                "{}{}",
                "0".repeat(leading_zeros),
                format_with_trimmed_zeros(stroops, min_decimals)
            );
            let parsed = parse_stroops(&formatted).unwrap();
            prop_assert_eq!(parsed, stroops);
            prop_assert_eq!(parsed, legacy_f64_stroops(&formatted));
        }

        /// Whole-unit strings (`<= 7` decimals with zero fractional stroops) have exact binary64
        /// integer representations all the way up to `2^53` stroops and match `f64` identically.
        #[test]
        fn prop_matches_legacy_f64_for_all_whole_units_below_2_pow_53_stroops(
            whole in 1i64..=((1i64 << 53) / STROOPS_PER_UNIT),
            trailing_zeros in 0usize..=7usize,
        ) {
            let s = if trailing_zeros == 0 {
                whole.to_string()
            } else {
                format!("{whole}.{}", "0".repeat(trailing_zeros))
            };
            let expected = whole * STROOPS_PER_UNIT;
            let parsed = parse_stroops(&s).unwrap();
            prop_assert_eq!(parsed, expected);
            prop_assert_eq!(parsed, legacy_f64_stroops(&s));
        }

        /// Across the full `1 .. 2^53` stroop range (and all the way to `i64::MAX`),
        /// `parse_stroops` recovers the exact integer stroop count without rounding.
        #[test]
        fn prop_roundtrips_every_valid_i64_stroop_value(
            stroops in 1i64..=i64::MAX,
            min_decimals in 0usize..=7usize,
        ) {
            let s = format_with_trimmed_zeros(stroops, min_decimals);
            prop_assert_eq!(parse_stroops(&s).unwrap(), stroops);
        }
    }
}
