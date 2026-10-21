//! International fiat allowlist for the card program (CARD-09).
//!
//! The card charges *international card-network fiat*, which is not the same
//! thing as the offramp corridor currency. NGN appears here because it is an
//! allowlisted charging currency in its own right; nothing in this module
//! touches the offramp NGN types in the frontend.
//!
//! This is the single source of truth for which ISO-4217 codes the card
//! program accepts and how many minor-unit decimals each code carries. The
//! validate handler consults [`validate`] rather than carrying its own list, so
//! a code can never be accepted by the route but unknown to the program.
//!
//! Additive and default-safe: this module only reads compile-time data and
//! performs no I/O, so importing it changes no live behavior on `/swap`,
//! `/offramp`, or `/cross-chain-swap`.

/// Allowlisted ISO-4217 code paired with its minor-unit decimal count.
///
/// JPY is the only zero-decimal entry; every other code is 2.
const ALLOWED_FIAT: [(&str, u8); 7] = [
    ("USD", 2),
    ("EUR", 2),
    ("GBP", 2),
    ("CAD", 2),
    ("AUD", 2),
    ("JPY", 0),
    ("NGN", 2),
];

/// Every allowlisted code, in declaration order.
pub fn allowed_codes() -> impl Iterator<Item = &'static str> {
    ALLOWED_FIAT.iter().map(|(code, _)| *code)
}

/// The allowlist as `(code, decimals)` pairs, in declaration order.
pub fn allowed_fiat() -> &'static [(&'static str, u8)] {
    &ALLOWED_FIAT
}

/// Resolve a submitted code to its canonical uppercase allowlist entry.
///
/// Matching is ASCII-case-insensitive so `"usd"` and `"USD"` agree, but the
/// returned code is always the canonical uppercase spelling.
///
/// Returns `None` for an empty or non-allowlisted code.
pub fn canonical(code: &str) -> Option<&'static str> {
    let trimmed = code.trim();
    ALLOWED_FIAT
        .iter()
        .find(|(allowed, _)| allowed.eq_ignore_ascii_case(trimmed))
        .map(|(allowed, _)| *allowed)
}

/// `true` when `code` is on the allowlist (ASCII-case-insensitive).
pub fn is_allowed(code: &str) -> bool {
    canonical(code).is_some()
}

/// Minor-unit decimal count for an allowlisted code.
///
/// JPY is `0`; USD, EUR, GBP, CAD, AUD, and NGN are `2`. Returns `None` for a
/// code that is not on the allowlist.
pub fn decimals(code: &str) -> Option<u8> {
    let trimmed = code.trim();
    ALLOWED_FIAT
        .iter()
        .find(|(allowed, _)| allowed.eq_ignore_ascii_case(trimmed))
        .map(|(_, decimals)| *decimals)
}

/// Validate a submitted currency code, returning the canonical spelling.
///
/// An omitted or blank code fails: the card always charges in a known
/// international fiat, so there is no implicit default.
///
/// # Errors
/// Returns a static message suitable for a `400` validation body when the code
/// is absent, blank, or not on the allowlist.
pub fn validate(code: Option<&str>) -> Result<&'static str, &'static str> {
    let Some(code) = code else {
        return Err("currency is required and must be one of USD, EUR, GBP, CAD, AUD, JPY, NGN");
    };
    if code.trim().is_empty() {
        return Err("currency is required and must be one of USD, EUR, GBP, CAD, AUD, JPY, NGN");
    }
    canonical(code).ok_or("currency must be one of USD, EUR, GBP, CAD, AUD, JPY, NGN")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact set the issue pins, in order.
    const EXPECTED: [(&str, u8); 7] = [
        ("USD", 2),
        ("EUR", 2),
        ("GBP", 2),
        ("CAD", 2),
        ("AUD", 2),
        ("JPY", 0),
        ("NGN", 2),
    ];

    #[test]
    fn allowlist_matches_the_pinned_set() {
        assert_eq!(allowed_fiat(), &EXPECTED);
        assert_eq!(allowed_codes().count(), 7);
        assert_eq!(
            allowed_codes().collect::<Vec<_>>(),
            vec!["USD", "EUR", "GBP", "CAD", "AUD", "JPY", "NGN"]
        );
    }

    /// Acceptance criterion: JPY decimals are 0 and USD decimals are 2.
    #[test]
    fn jpy_is_zero_decimal_and_usd_is_two() {
        assert_eq!(decimals("JPY"), Some(0));
        assert_eq!(decimals("USD"), Some(2));
    }

    /// Acceptance criterion: unit test every code.
    #[test]
    fn every_code_reports_its_decimals() {
        for (code, expected) in EXPECTED {
            assert_eq!(decimals(code), Some(expected), "decimals for {code}");
            assert!(is_allowed(code), "{code} must be allowlisted");
            assert_eq!(validate(Some(code)), Ok(code), "validate for {code}");
        }
    }

    /// Every allowlisted code is 0 or 2 decimals, and JPY is the only 0.
    #[test]
    fn only_jpy_is_zero_decimal() {
        let zero: Vec<&str> = allowed_codes().filter(|c| decimals(c) == Some(0)).collect();
        assert_eq!(zero, vec!["JPY"]);
    }

    /// NGN is allowlisted for card charging, independent of the offramp types.
    #[test]
    fn ngn_is_allowlisted() {
        assert!(is_allowed("NGN"));
        assert_eq!(decimals("NGN"), Some(2));
    }

    /// Acceptance criterion: an omitted code fails validation.
    #[test]
    fn omitted_code_fails_validation() {
        assert!(validate(None).is_err());
        assert!(validate(Some("")).is_err());
        assert!(validate(Some("   ")).is_err());
    }

    #[test]
    fn unknown_code_fails_validation() {
        for bad in ["CHF", "US", "USDD", "ZZZ", "US D", "USDT", "€"] {
            assert!(validate(Some(bad)).is_err(), "{bad} must be rejected");
            assert!(!is_allowed(bad), "{bad} must not be allowlisted");
            assert_eq!(decimals(bad), None, "{bad} must have no decimals");
        }
    }

    #[test]
    fn lookup_is_case_insensitive_and_returns_canonical_case() {
        assert_eq!(canonical("usd"), Some("USD"));
        assert_eq!(canonical("jpy"), Some("JPY"));
        assert_eq!(canonical("Ngn"), Some("NGN"));
        assert_eq!(decimals("jpy"), Some(0));
        assert_eq!(validate(Some("eur")), Ok("EUR"));
    }

    #[test]
    fn surrounding_whitespace_is_ignored() {
        assert_eq!(validate(Some("  USD  ")), Ok("USD"));
        assert_eq!(decimals(" GBP "), Some(2));
    }

    #[test]
    fn allowlist_has_no_duplicate_codes() {
        let mut seen = allowed_codes().collect::<Vec<_>>();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), before, "allowlist must not repeat a code");
    }
}
