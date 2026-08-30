//! Normalized venue classification for classic SDEX prepare/submit.
//!
//! All prepare/kill-switch/audit paths MUST use [`classify_venue`] so mode
//! selection cannot disagree with pause checks via ad-hoc `contains` matching.

use stellarroute_routing::health::scorer::VenueType;

/// Explicit venue class used by swap prepare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapVenueClass {
    /// Classic Stellar DEX (orderbook / path payment).
    Sdex,
    /// Soroban AMM / router — not supported by the classic prepare path.
    AmmOrRouter,
    /// Unrecognized venue string.
    Unknown,
}

/// Classify a hop's `source` / `venue_ref` into a stable venue class.
///
/// Supported classic forms (case-insensitive, trimmed):
/// - `sdex`
/// - `sdex:<anything>`
/// - `horizon`
/// - `horizon:<anything>`
///
/// Explicit AMM/router forms (rejected by prepare):
/// - `amm`, `amm:<…>`, `soroban`, `soroban:<…>`, `router`, `router:<…>`
pub fn classify_venue(source: &str, venue_ref: Option<&str>) -> SwapVenueClass {
    let primary = normalize_token(source);
    if primary.is_empty() {
        if let Some(vr) = venue_ref {
            return classify_token(&normalize_token(vr));
        }
        return SwapVenueClass::Unknown;
    }
    let class = classify_token(&primary);
    if class != SwapVenueClass::Unknown {
        return class;
    }
    if let Some(vr) = venue_ref {
        return classify_token(&normalize_token(vr));
    }
    SwapVenueClass::Unknown
}

fn normalize_token(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

fn classify_token(token: &str) -> SwapVenueClass {
    let head = token.split(':').next().unwrap_or(token);
    match head {
        "sdex" | "horizon" => SwapVenueClass::Sdex,
        "amm" | "soroban" | "router" => SwapVenueClass::AmmOrRouter,
        _ => SwapVenueClass::Unknown,
    }
}

impl SwapVenueClass {
    pub fn to_routing_venue_type(self) -> Option<VenueType> {
        match self {
            Self::Sdex => Some(VenueType::Sdex),
            Self::AmmOrRouter => Some(VenueType::Amm),
            Self::Unknown => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sdex => "sdex",
            Self::AmmOrRouter => "amm",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_explicit_sdex_forms() {
        assert_eq!(classify_venue("sdex", None), SwapVenueClass::Sdex);
        assert_eq!(
            classify_venue("SDEX", Some("sdex-venue")),
            SwapVenueClass::Sdex
        );
        assert_eq!(classify_venue("sdex:123", None), SwapVenueClass::Sdex);
        assert_eq!(classify_venue("horizon", None), SwapVenueClass::Sdex);
    }

    #[test]
    fn classifies_amm_and_router_as_unsupported() {
        assert_eq!(classify_venue("amm", None), SwapVenueClass::AmmOrRouter);
        assert_eq!(
            classify_venue("amm:pool", None),
            SwapVenueClass::AmmOrRouter
        );
        assert_eq!(classify_venue("soroban", None), SwapVenueClass::AmmOrRouter);
        assert_eq!(classify_venue("router", None), SwapVenueClass::AmmOrRouter);
        // Must not treat substring "amm" inside unrelated tokens as SDEX.
        assert_eq!(
            classify_venue("programmable", None),
            SwapVenueClass::Unknown
        );
    }

    #[test]
    fn does_not_use_contains_false_positives() {
        // "sdex" as substring of something else is not accepted.
        assert_eq!(classify_venue("mysdexpool", None), SwapVenueClass::Unknown);
        assert_eq!(classify_venue("hammer", None), SwapVenueClass::Unknown);
    }
}
