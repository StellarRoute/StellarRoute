//! CARD-03 partner configuration reporting for card health (issue #1463).
//!
//! Additive-only: this module answers a single question - is the partner
//! Stellar account configured? - so `GET /api/v1/card/health` can report
//! `issuer: partner_unconfigured` while `CARD_PARTNER_STELLAR_ADDRESS` is
//! unset or empty. No keys, PANs, or secrets are read or exposed; the
//! partner address is a public G-address.

/// Sentinel returned in the card health `issuer` field while the partner
/// Stellar address is missing or empty.
pub const PARTNER_UNCONFIGURED: &str = "partner_unconfigured";

/// Resolve the health `issuer` label from a raw partner address value.
///
/// Empty or whitespace-only input yields [`PARTNER_UNCONFIGURED`]; anything
/// else is returned trimmed (the configured public partner G-address).
pub fn issuer_for(partner_address: Option<&str>) -> String {
    match partner_address.map(str::trim) {
        Some(addr) if !addr.is_empty() => addr.to_string(),
        _ => PARTNER_UNCONFIGURED.to_string(),
    }
}

/// Read `CARD_PARTNER_STELLAR_ADDRESS` from the environment and resolve the
/// health `issuer` label. Evaluated per request so operators can observe a
/// configuration change without a redeploy, mirroring `routes::card::card_enabled`.
pub fn issuer() -> String {
    issuer_for(
        std::env::var(crate::card::CARD_PARTNER_STELLAR_ADDRESS_ENV)
            .ok()
            .as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_partner_address_reports_partner_unconfigured() {
        assert_eq!(issuer_for(None), PARTNER_UNCONFIGURED);
    }

    #[test]
    fn empty_and_whitespace_partner_address_report_partner_unconfigured() {
        assert_eq!(issuer_for(Some("")), PARTNER_UNCONFIGURED);
        assert_eq!(issuer_for(Some("   ")), PARTNER_UNCONFIGURED);
        assert_eq!(issuer_for(Some("\t\n")), PARTNER_UNCONFIGURED);
    }

    #[test]
    fn configured_partner_address_is_returned_trimmed() {
        assert_eq!(
            issuer_for(Some(" GTESTPARTNERADDRESS ")),
            "GTESTPARTNERADDRESS"
        );
    }

    #[test]
    fn env_missing_or_empty_falls_back_to_sentinel() {
        let key = crate::card::CARD_PARTNER_STELLAR_ADDRESS_ENV;
        let prev = std::env::var(key).ok();

        std::env::remove_var(key);
        assert_eq!(issuer(), PARTNER_UNCONFIGURED);

        std::env::set_var(key, "");
        assert_eq!(issuer(), PARTNER_UNCONFIGURED);

        std::env::set_var(key, "GTESTPARTNERADDRESS");
        assert_eq!(issuer(), "GTESTPARTNERADDRESS");

        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }
}
