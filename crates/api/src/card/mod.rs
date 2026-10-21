//! Card program (stablecoin debit) — additive, default-off.
//!
//! Every route in this module is gated by `CARD_ENABLED`. When the flag is
//! unset or false the routes answer `404`, exactly like an unknown path, so
//! nothing changes for live users of `/swap`, `/offramp`, or
//! `/cross-chain-swap`. StellarRoute never holds keys or card PANs here: the
//! user signs the partner payment in their own wallet and the partner owns the
//! card credentials.

pub mod application;
pub mod auth_state;
pub mod authorization;
pub mod errors;
pub mod fx;
pub mod horizon;
pub mod limits;
pub mod mcc;
pub mod store;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::models::{ApiErrorCode, ErrorResponse};

use store::{
    ApplicationRecord, AuthorizationRecord, CardStore, InMemoryCardStore, PartnerEventRecord,
    StoreError,
};

pub use auth_state::{
    apply_event, transition, AuthorizationEvent, AuthorizationState, TransitionError,
};

/// Master card feature flag. Defaults to off.
pub const CARD_ENABLED_ENV: &str = "CARD_ENABLED";
/// Partner-owned Stellar account that receives the USDC spend.
pub const CARD_PARTNER_STELLAR_ADDRESS_ENV: &str = "CARD_PARTNER_STELLAR_ADDRESS";
/// Optional USDC issuer pin for the partner payment.
pub const CARD_USDC_ISSUER_ENV: &str = "CARD_USDC_ISSUER";

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// `true` only for an explicit truthy `CARD_ENABLED`.
pub fn is_card_enabled() -> bool {
    env_nonempty(CARD_ENABLED_ENV)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Card runtime configuration, resolved once when the router is built.
#[derive(Debug, Clone, Default)]
pub struct CardConfig {
    pub enabled: bool,
    /// Raw `CARD_WEBHOOK_HMAC_KEY` bytes. Only the partner webhook reads it.
    pub webhook_hmac_key: Option<Vec<u8>>,
    /// `CARD_PARTNER_STELLAR_ADDRESS`; authorizations fail closed without it.
    pub partner_address: Option<String>,
    /// Optional `CARD_USDC_ISSUER` pin.
    pub usdc_issuer: Option<String>,
    pub network_passphrase: String,
}

impl CardConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: is_card_enabled(),
            webhook_hmac_key: None,
            partner_address: env_nonempty(CARD_PARTNER_STELLAR_ADDRESS_ENV),
            usdc_issuer: env_nonempty(CARD_USDC_ISSUER_ENV),
            network_passphrase: crate::swap::tx::network_passphrase_from_env(),
        }
    }
}

/// Shared state for the card routes.
#[derive(Clone)]
pub struct CardState {
    pub config: CardConfig,
    pub store: Arc<dyn CardStore>,
    frozen: Arc<AtomicBool>,
}

impl CardState {
    pub fn from_env() -> Self {
        Self {
            config: CardConfig::from_env(),
            store: Arc::new(InMemoryCardStore::default()),
            frozen: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen.load(Ordering::Acquire)
    }

    pub fn freeze(&self) {
        self.frozen.store(true, Ordering::Release);
    }

    pub fn unfreeze(&self) {
        self.frozen.store(false, Ordering::Release);
    }
}

#[derive(Debug, Deserialize)]
struct CardAuthorizationRequest {
    authorization_id: String,
    tx_hash: String,
    amount_stroops: i64,
    #[serde(default)]
    fiat_amount: Option<i64>,
    #[serde(default)]
    fiat_currency: Option<String>,
    #[serde(default)]
    rate: Option<f64>,
    #[serde(default)]
    rate_locked_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PartnerEventRequest {
    event_id: String,
    event_type: String,
    authorization_id: String,
    #[serde(default)]
    amount_stroops: Option<i64>,
    #[serde(default)]
    fiat_amount: Option<i64>,
    #[serde(default)]
    fiat_currency: Option<String>,
    #[serde(default)]
    rate: Option<f64>,
    #[serde(default)]
    rate_locked_at: Option<i64>,
    #[serde(default)]
    payload: Option<serde_json::Value>,
}

async fn freeze_card(State(state): State<Arc<CardState>>) -> Response {
    if !state.config.enabled {
        return not_found();
    }
    state.freeze();
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "frozen", "frozen": true })),
    )
        .into_response()
}

async fn unfreeze_card(State(state): State<Arc<CardState>>) -> Response {
    if !state.config.enabled {
        return not_found();
    }
    state.unfreeze();
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "active", "frozen": false })),
    )
        .into_response()
}

async fn list_authorizations(State(state): State<Arc<CardState>>) -> Response {
    if !state.config.enabled {
        return not_found();
    }

    let rows = state.store.list_authorizations();
    (StatusCode::OK, Json(rows)).into_response()
}

async fn handle_partner_event(
    State(state): State<Arc<CardState>>,
    Json(payload): Json<PartnerEventRequest>,
) -> Response {
    if !state.config.enabled {
        return not_found();
    }

    let event_record = PartnerEventRecord {
        event_id: payload.event_id.clone(),
        event_type: Some(payload.event_type.clone()),
        payload: serde_json::json!({
            "authorization_id": payload.authorization_id,
            "event_id": payload.event_id,
            "event_type": payload.event_type,
            "amount_stroops": payload.amount_stroops,
            "payload": payload.payload
        }),
    };

    if !state.store.insert_partner_event(event_record.clone()) {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "duplicate",
                "event_id": event_record.event_id,
                "event_type": payload.event_type,
                "authorization_id": payload.authorization_id,
                "message": "Partner event already processed; no-op success."
            })),
        )
            .into_response();
    }

    let result = match payload.event_type.as_str() {
        "authorization" => {
            state.store.authorization(&payload.authorization_id).map_or_else(
                || {
                    state.store.approve_and_hold_with_fx(
                        &payload.authorization_id,
                        "partner-event",
                        payload.amount_stroops.unwrap_or(0),
                        payload.fiat_amount.unwrap_or(0),
                        payload.fiat_currency.as_deref().unwrap_or(""),
                        payload.rate.unwrap_or(0.0),
                        payload.rate_locked_at.unwrap_or(0),
                    )
                },
                Ok,
            )
        }
        "clearing" => {
            let auth = match state.store.authorization(&payload.authorization_id) {
                Some(auth) => auth,
                None => return card_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "authorization_not_found",
                    "Authorization was not found for clearing.",
                ),
            };
            let webhook_fiat = payload.fiat_amount.unwrap_or(auth.fiat_amount);
            if (webhook_fiat - auth.fiat_amount).abs() > 1 {
                return card_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "fiat_amount_mismatch",
                    "Webhook fiat amount differs from the locked authorization by more than one minor unit.",
                );
            }
            let _ = auth.rate;
            state.store.capture_hold(
                &payload.authorization_id,
                payload.amount_stroops.unwrap_or(0),
            )
        }
        "reversal" => state.store.reverse_hold(&payload.authorization_id),
        "refund" => state.store.refund_hold(
            &payload.authorization_id,
            payload.amount_stroops.unwrap_or(0),
        ),
        _ => Err(StoreError::UnsupportedEventType),
    };

    match result {
        Ok(record) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "applied",
                "event_id": event_record.event_id,
                "event_type": payload.event_type,
                "authorization": record,
                "held_stroops": state.store.held_stroops(&record.authorization_id),
            })),
        )
            .into_response(),
        Err(StoreError::UnsupportedEventType) => card_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unknown_event_type",
            "Unsupported partner event type.",
        ),
        Err(StoreError::AmountExceedsAvailable) => card_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "amount_exceeds_available",
            "Captured amount exceeds the available hold.",
        ),
        Err(StoreError::RefundExceedsCaptured) => card_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "refund_exceeds_captured",
            "Refund exceeds the amount already captured.",
        ),
        Err(StoreError::FiatAmountMismatch) => card_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "fiat_amount_mismatch",
            "Webhook fiat amount differs from the locked authorization by more than one minor unit.",
        ),
        Err(StoreError::HoldNotFound) => card_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "hold_not_found",
            "No active hold exists to reverse.",
        ),
        Err(_) => card_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "partner_event_failed",
            "Partner event could not be applied.",
        ),
    }
}

async fn record_authorization(
    State(state): State<Arc<CardState>>,
    Json(payload): Json<CardAuthorizationRequest>,
) -> Response {
    if !state.config.enabled {
        return not_found();
    }
    if state.is_frozen() {
        return card_error(
            StatusCode::FORBIDDEN,
            "frozen",
            "Card program is frozen; new authorizations are rejected.",
        );
    }

    match state.store.approve_and_hold_with_fx(
        &payload.authorization_id,
        &payload.tx_hash,
        payload.amount_stroops,
        payload.fiat_amount.unwrap_or(0),
        payload.fiat_currency.as_deref().unwrap_or(""),
        payload.rate.unwrap_or(0.0),
        payload.rate_locked_at.unwrap_or(0),
    ) {
        Ok(record) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "authorization": record,
                "status": "approved",
                "held_stroops": state.store.held_stroops(&record.authorization_id),
            })),
        )
            .into_response(),
        Err(StoreError::AlreadyApproved) => card_error(
            StatusCode::CONFLICT,
            "already_approved",
            "Authorization is already approved.",
        ),
        Err(StoreError::TxAlreadyUsed) => card_error(
            StatusCode::CONFLICT,
            "tx_already_used",
            "This transaction hash has already been used.",
        ),
        Err(_) => card_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "authorization_failed",
            "Authorization could not be recorded.",
        ),
    }
}

/// `404` returned by every card route while the flag is off.
pub(crate) fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(ApiErrorCode::NotFound, "Not found")),
    )
        .into_response()
}

/// Card error body: `{ "error": "<code>", "message": "..." }`.
pub(crate) fn card_error(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    (
        status,
        Json(serde_json::json!({ "error": code, "message": message.into() })),
    )
        .into_response()
}

/// Card routes bound to an explicit state (used by tests).
pub fn router_with_state(state: Arc<CardState>) -> Router {
    Router::new()
        .route("/api/v1/card/freeze", post(freeze_card))
        .route("/api/v1/card/unfreeze", post(unfreeze_card))
        .route(
            "/api/v1/card/authorizations",
            get(list_authorizations).post(record_authorization),
        )
        .route(
            "/api/v1/card/partner/events",
            post(handle_partner_event),
        )
        .with_state(state)
}

/// Card routes configured from the environment.
pub fn router() -> Router {
    router_with_state(Arc::new(CardState::from_env()))
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    pub async fn send(
        app: Router,
        path: &str,
        headers: &[(&str, &str)],
        body: impl Into<Body>,
    ) -> (StatusCode, serde_json::Value) {
        let mut req = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json");
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let resp = app.oneshot(req.body(body.into()).unwrap()).await.unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_disabled() {
        assert!(!CardConfig::default().enabled);
    }
}
