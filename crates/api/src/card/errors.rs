//! Card-specific error codes and HTTP status mappings.
//!
//! Define: insufficient_stable_balance, limit_exceeded, frozen, kyc_required,
//! fx_expired, country_blocked, partner_unconfigured. Map each to an HTTP
//! status on card routes only.

use axum::http::StatusCode;

/// Card-specific error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardErrorCode {
    /// Insufficient balance on the stable account.
    InsufficientStableBalance,
    /// Transaction exceeds the card spending limit.
    LimitExceeded,
    /// Card account is frozen.
    Frozen,
    /// KYC verification required.
    KycRequired,
    /// FX quote or conversion expired.
    FxExpired,
    /// User's country is blocked from card operations.
    CountryBlocked,
    /// Partner configuration is missing or incomplete.
    PartnerUnconfigured,
}

impl CardErrorCode {
    /// Return the error code as a string for JSON responses.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InsufficientStableBalance => "insufficient_stable_balance",
            Self::LimitExceeded => "limit_exceeded",
            Self::Frozen => "frozen",
            Self::KycRequired => "kyc_required",
            Self::FxExpired => "fx_expired",
            Self::CountryBlocked => "country_blocked",
            Self::PartnerUnconfigured => "partner_unconfigured",
        }
    }

    /// Map the error code to an HTTP status code.
    ///
    /// - 400 Bad Request: limit_exceeded, fx_expired, country_blocked
    /// - 402 Payment Required: insufficient_stable_balance
    /// - 403 Forbidden: frozen, kyc_required
    /// - 503 Service Unavailable: partner_unconfigured
    pub fn http_status(&self) -> StatusCode {
        match self {
            Self::InsufficientStableBalance => StatusCode::PAYMENT_REQUIRED, // 402
            Self::LimitExceeded => StatusCode::BAD_REQUEST,                  // 400
            Self::Frozen => StatusCode::FORBIDDEN,                           // 403
            Self::KycRequired => StatusCode::FORBIDDEN,                      // 403
            Self::FxExpired => StatusCode::BAD_REQUEST,                      // 400
            Self::CountryBlocked => StatusCode::BAD_REQUEST,                 // 400
            Self::PartnerUnconfigured => StatusCode::SERVICE_UNAVAILABLE,    // 503
        }
    }
}

/// Error body format for card routes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CardError {
    pub error: String,
    pub message: String,
}

impl CardError {
    pub fn new(code: CardErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: code.as_str().to_string(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_have_correct_status_mappings() {
        assert_eq!(
            CardErrorCode::InsufficientStableBalance.http_status(),
            StatusCode::PAYMENT_REQUIRED
        );
        assert_eq!(
            CardErrorCode::LimitExceeded.http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(CardErrorCode::Frozen.http_status(), StatusCode::FORBIDDEN);
        assert_eq!(
            CardErrorCode::KycRequired.http_status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            CardErrorCode::FxExpired.http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            CardErrorCode::CountryBlocked.http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            CardErrorCode::PartnerUnconfigured.http_status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn error_codes_serialize_correctly() {
        let err = CardError::new(
            CardErrorCode::LimitExceeded,
            "You have exceeded your daily limit",
        );
        assert_eq!(err.error, "limit_exceeded");
        assert_eq!(err.message, "You have exceeded your daily limit");
    }

    #[test]
    fn all_error_codes_have_string_representation() {
        let codes = [
            CardErrorCode::InsufficientStableBalance,
            CardErrorCode::LimitExceeded,
            CardErrorCode::Frozen,
            CardErrorCode::KycRequired,
            CardErrorCode::FxExpired,
            CardErrorCode::CountryBlocked,
            CardErrorCode::PartnerUnconfigured,
        ];
        for code in &codes {
            let s = code.as_str();
            assert!(!s.is_empty());
            assert!(s.chars().all(|c| c.is_lowercase() || c == '_'));
        }
    }
}
