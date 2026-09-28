//! Card payment routes
//!
//! These routes are only registered when CARD_ENABLED=true
//!
//! Card program routes — additive, flag-gated preview surfaces (CARD-38).
//!
//! These endpoints are intentionally minimal and fail-closed:
//! - When `CARD_ENABLED` is unset or anything other than `"true"`, every
//!   handler returns `404 not_found` (`card disabled`). StellarRoute never
//!   holds API keys or card PANs; no PAN, card number, CVV/CVC, or expiry
//!   field exists on any request or response schema here.
//! - When `CARD_ENABLED=true`, handlers return static, non-sensitive shapes
//!   so SDK and frontend integrators can build against a stable contract
//!   without touching the live swap/quote path.
//!
//! Nothing in this module touches:
//! - classic one-hop SDEX prepare → sign → submit (`routes::swap`),
//! - quote selection / ranking (`routes::quote`),
//! - CORS allowlists, `CCTP_ENABLED` default, or existing OpenAPI field names.

use axum::{routing::post, Router};
use serde_json::json;
use std::sync::Arc;

use crate::state::AppState;

/// Create card routes router
pub fn create_card_router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/card/webhook", post(card_webhook))
        .with_state(state)
}

/// Card webhook handler for partner events
async fn card_webhook(
    axum::extract::State(_state): axum::extract::State<Arc<AppState>>,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, crate::error::ApiError> {
    use crate::card::webhooks::{deserialize_partner_event, PartnerEvent};

    let payload_str = serde_json::to_string(&payload)
        .map_err(|e| crate::error::ApiError::BadRequest(format!("Invalid JSON payload: {}", e)))?;

    let event = deserialize_partner_event(&payload_str)?;

    // Process the event based on type
    match event {
        PartnerEvent::Authorization(ref auth) => {
            tracing::info!(
                event_id = %auth.event_id,
                partner_txn_id = %auth.partner_transaction_id,
                amount = auth.amount,
                currency = %auth.currency,
                "Card authorization received"
            );
        }
        PartnerEvent::Clearing(ref clearing) => {
            tracing::info!(
                event_id = %clearing.event_id,
                partner_txn_id = %clearing.partner_transaction_id,
                amount = clearing.amount,
                currency = %clearing.currency,
                "Card clearing received"
            );
        }
        PartnerEvent::Reversal(ref reversal) => {
            tracing::info!(
                event_id = %reversal.event_id,
                partner_txn_id = %reversal.partner_transaction_id,
                amount = reversal.amount,
                currency = %reversal.currency,
                "Card reversal received"
            );
        }
    }

    let event_id = match event {
        PartnerEvent::Authorization(a) => a.event_id,
        PartnerEvent::Clearing(c) => c.event_id,
        PartnerEvent::Reversal(r) => r.event_id,
    };

    Ok(axum::Json(json!({
        "status": "accepted",
        "event_id": event_id
    })))
}

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{error::ApiError, models::ApiResponse};

/// International fiat allowlist (CARD-09, issue #1469).
///
/// Included by path because `card` is not yet declared in `lib.rs`, and
/// declaring it there is not an option while sibling card modules
/// (`authorization`, `fx`, `horizon`, `webhook`) are still unimplemented.
/// `crate::routes::card` is public, so everything in here stays reachable and
/// nothing in this module is reported as dead code.
#[path = "../card/fiat.rs"]
pub mod fiat;

/// Returns `true` only when `CARD_ENABLED=true` (case-insensitive, trimmed).
///
/// Unset, empty, or any other value means disabled. This keeps the new
/// routes invisible (`404`) in production unless an operator explicitly opts
/// in — the same fail-closed posture as `CCTP_ENABLED`.
pub fn card_enabled() -> bool {
    std::env::var("CARD_ENABLED")
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn card_disabled() -> ApiError {
    ApiError::NotFound("card disabled".to_string())
}

// ── Schemas (no PANs, no secrets) ────────────────────────────────────────────

/// Card program health — `enabled: false` is the 404-disabled shape the
/// JS/Rust SDKs normalize to instead of surfacing an error.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CardHealth {
    /// Whether the card program is enabled on this deployment.
    pub enabled: bool,
}

/// Minimal application draft. Integrators POST a caller-chosen reference and
/// display name only — never a card number, expiry, or CVV.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CardApplicationDraft {
    /// Caller-chosen idempotency / applicant reference (opaque string).
    pub applicant_ref: String,
    /// Display name for the application (not a PAN, not a key).
    #[serde(default)]
    pub display_name: Option<String>,
    /// ISO-4217 charging currency for the card (CARD-09).
    ///
    /// Optional on the wire so previously valid payloads still deserialize,
    /// but the validate handler requires it: an omitted code fails
    /// validation. Must be one of the codes in [`fiat`].
    #[serde(default)]
    pub currency: Option<String>,
}

/// Validation outcome for a draft application.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CardApplicationValidation {
    /// Whether the draft passed validation.
    pub valid: bool,
    /// Echo of the submitted applicant reference.
    pub applicant_ref: String,
}

/// A single card authorization (webhook-derived view model).
///
/// `decline_code` is a short machine code (e.g. `insufficient_funds`);
/// frontends map it to plain language. No PAN fields are present by design.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CardAuthorization {
    /// Stable authorization identifier; dismiss state keys off this.
    pub id: String,
    /// Authorization status: `approved`, `declined`, or `pending`.
    pub status: String,
    /// Machine-readable decline code when `status == "declined"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decline_code: Option<String>,
    /// Decimal amount string (e.g. `"12.50"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    /// ISO-4217 currency code (e.g. `"USD"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// RFC-3339 timestamp when the authorization was created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// List response for card authorizations.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CardAuthorizationsResponse {
    pub authorizations: Vec<CardAuthorization>,
    pub total: usize,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/v1/card/health
#[utoipa::path(
    get,
    path = "/api/v1/card/health",
    tag = "card",
    responses(
        (status = 200, description = "Card program health", body = ApiResponse<CardHealth>),
        (status = 404, description = "Card program disabled", body = crate::models::ErrorResponse),
    )
)]
pub async fn card_health(
    State(_state): State<Arc<AppState>>,
    request_id: crate::middleware::RequestId,
) -> Result<impl IntoResponse, ApiError> {
    if !card_enabled() {
        return Err(card_disabled());
    }
    let body = ApiResponse::new(
        CardHealth { enabled: true },
        request_id.as_str().to_string(),
    );
    Ok((StatusCode::OK, Json(body)))
}

/// POST /api/v1/card/applications/validate
#[utoipa::path(
    post,
    path = "/api/v1/card/applications/validate",
    tag = "card",
    request_body(content = CardApplicationDraft, description = "Draft card application to validate"),
    responses(
        (status = 200, description = "Validation outcome", body = ApiResponse<CardApplicationValidation>),
        (status = 400, description = "Validation error", body = crate::models::ErrorResponse),
        (status = 404, description = "Card program disabled", body = crate::models::ErrorResponse),
    )
)]
pub async fn validate_card_application(
    State(_state): State<Arc<AppState>>,
    request_id: crate::middleware::RequestId,
    Json(draft): Json<CardApplicationDraft>,
) -> Result<impl IntoResponse, ApiError> {
    if !card_enabled() {
        return Err(card_disabled());
    }
    if draft.applicant_ref.trim().is_empty() {
        return Err(ApiError::Validation(
            "applicant_ref must not be empty".to_string(),
        ));
    }
    // CARD-09: the card charges international card-network fiat, so the code
    // is required and must be on the centralized allowlist. An omitted code
    // fails rather than defaulting.
    if let Err(message) = fiat::validate(draft.currency.as_deref()) {
        return Err(ApiError::Validation(message.to_string()));
    }
    let body = ApiResponse::new(
        CardApplicationValidation {
            valid: true,
            applicant_ref: draft.applicant_ref,
        },
        request_id.as_str().to_string(),
    );
    Ok((StatusCode::OK, Json(body)))
}

/// GET /api/v1/card/authorizations
#[utoipa::path(
    get,
    path = "/api/v1/card/authorizations",
    tag = "card",
    responses(
        (status = 200, description = "Recent card authorizations", body = ApiResponse<CardAuthorizationsResponse>),
        (status = 404, description = "Card program disabled", body = crate::models::ErrorResponse),
    )
)]
pub async fn list_card_authorizations(
    State(_state): State<Arc<AppState>>,
    request_id: crate::middleware::RequestId,
) -> Result<impl IntoResponse, ApiError> {
    if !card_enabled() {
        return Err(card_disabled());
    }
    // Empty by default; integrators render the paused sentence when there is
    // nothing declined to show. No seed data, no PANs.
    let body = ApiResponse::new(
        CardAuthorizationsResponse {
            authorizations: Vec::new(),
            total: 0,
        },
        request_id.as_str().to_string(),
    );
    Ok((StatusCode::OK, Json(body)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_disabled_by_default_when_env_unset() {
        let prev = std::env::var("CARD_ENABLED").ok();
        std::env::remove_var("CARD_ENABLED");
        assert!(!card_enabled());
        if let Some(v) = prev {
            std::env::set_var("CARD_ENABLED", v);
        }
    }

    #[test]
    fn card_enabled_only_on_true() {
        let prev = std::env::var("CARD_ENABLED").ok();
        std::env::set_var("CARD_ENABLED", "true");
        assert!(card_enabled());
        std::env::set_var("CARD_ENABLED", "false");
        assert!(!card_enabled());
        std::env::set_var("CARD_ENABLED", "1");
        assert!(!card_enabled());
        match prev {
            Some(v) => std::env::set_var("CARD_ENABLED", v),
            None => std::env::remove_var("CARD_ENABLED"),
        }
    }

    #[test]
    fn card_schemas_carry_no_pan_fields() {
        let health = serde_json::to_value(CardHealth { enabled: true }).unwrap();
        let draft = serde_json::to_value(CardApplicationDraft {
            applicant_ref: "ref-1".into(),
            display_name: Some("Ada".into()),
            currency: Some("USD".into()),
        })
        .unwrap();
        let auth = serde_json::to_value(CardAuthorization {
            id: "auth-1".into(),
            status: "declined".into(),
            decline_code: Some("insufficient_funds".into()),
            amount: Some("12.50".into()),
            currency: Some("USD".into()),
            created_at: Some("2026-09-25T00:00:00Z".into()),
        })
        .unwrap();
        for v in [health, draft, auth] {
            let s = serde_json::to_string(&v).unwrap().to_lowercase();
            for forbidden in [
                "pan",
                "card_number",
                "cardnumber",
                "cvv",
                "cvc",
                "expiry",
                "exp_month",
            ] {
                assert!(
                    !s.contains(forbidden),
                    "card schema must not contain {forbidden}: {s}"
                );
            }
        }
    }

    // ── CARD-09: fiat allowlist wiring ──────────────────────────────────────

    /// A payload written before CARD-09 must still deserialize, so previously
    /// valid requests are not broken at the wire boundary. The missing code is
    /// rejected by validation instead of by deserialization.
    #[test]
    fn draft_without_currency_still_deserializes() {
        let draft: CardApplicationDraft =
            serde_json::from_str(r#"{"applicant_ref":"ref-1"}"#).unwrap();
        assert_eq!(draft.applicant_ref, "ref-1");
        assert_eq!(draft.currency, None);
        assert!(
            fiat::validate(draft.currency.as_deref()).is_err(),
            "omitted currency must fail validation"
        );
    }

    #[test]
    fn draft_accepts_every_allowlisted_currency() {
        for code in fiat::allowed_codes() {
            let raw = format!(r#"{{"applicant_ref":"ref-1","currency":"{code}"}}"#);
            let draft: CardApplicationDraft = serde_json::from_str(&raw).unwrap();
            assert_eq!(
                fiat::validate(draft.currency.as_deref()),
                Ok(code),
                "{code} must validate"
            );
        }
    }

    #[test]
    fn draft_rejects_non_allowlisted_currency() {
        let draft: CardApplicationDraft =
            serde_json::from_str(r#"{"applicant_ref":"ref-1","currency":"CHF"}"#).unwrap();
        assert!(fiat::validate(draft.currency.as_deref()).is_err());
    }

    /// JPY is the zero-decimal entry; USD is 2. Surfaced here so the route
    /// module's tests pin the same pair the allowlist unit tests do.
    #[test]
    fn route_exposes_expected_decimals() {
        assert_eq!(fiat::decimals("JPY"), Some(0));
        assert_eq!(fiat::decimals("USD"), Some(2));
    }
}
