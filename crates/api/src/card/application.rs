//! Card application validation (CARD-08).
//!
//! Validates application drafts without persisting them:
//! - POST /api/v1/card/applications/validate
//! - Returns 404 when CARD_ENABLED is off
//! - Returns 200 with normalized draft for valid inputs (USD, EUR, GBP)
//! - Returns 422 for unknown currencies (e.g. "XXX") or invalid fields

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use super::{card_error, not_found, CardState};

pub const ALLOWED_CURRENCIES: &[&str] = &["USD", "EUR", "GBP"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateApplicationRequest {
    pub name: Option<String>,
    pub country: Option<String>,
    #[serde(alias = "preferred_currency", alias = "fiat_currency")]
    pub currency: Option<String>,
    #[serde(alias = "monthly_limit", alias = "limit")]
    pub monthly_limit_usdc: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedApplicationDraft {
    pub name: String,
    pub country: String,
    pub currency: String,
    pub monthly_limit_usdc: f64,
}

pub async fn validate_application(
    State(state): State<Arc<CardState>>,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    if !state.config.enabled {
        return not_found();
    }

    let req: ValidateApplicationRequest = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => {
            return card_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_payload",
                format!("Failed to parse application draft: {e}"),
            );
        }
    };

    let name = match req.name {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => {
            return card_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_name",
                "Name is required and cannot be empty",
            );
        }
    };

    let country = match req.country {
        Some(c) if !c.trim().is_empty() && c.trim().len() >= 2 => c.trim().to_uppercase(),
        _ => {
            return card_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_country",
                "Country must be a valid country code (at least 2 letters)",
            );
        }
    };

    let raw_currency = match req.currency {
        Some(curr) if !curr.trim().is_empty() => curr.trim().to_uppercase(),
        _ => {
            return card_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_currency",
                "Currency is required",
            );
        }
    };

    if !ALLOWED_CURRENCIES.contains(&raw_currency.as_str()) {
        return card_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unsupported_currency",
            format!(
                "Currency '{raw_currency}' is not supported. Allowed: {}",
                ALLOWED_CURRENCIES.join(", ")
            ),
        );
    }

    let monthly_limit_usdc = match req.monthly_limit_usdc {
        Some(limit) if limit > 0.0 => limit,
        _ => {
            return card_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_limit",
                "Monthly USDC limit must be greater than zero",
            );
        }
    };

    let normalized = NormalizedApplicationDraft {
        name,
        country,
        currency: raw_currency,
        monthly_limit_usdc,
    };

    (StatusCode::OK, Json(normalized)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{test_support::send, CardConfig, CardStore, InMemoryCardStore};
    use serde_json::json;

    fn test_app(enabled: bool) -> (axum::Router, Arc<InMemoryCardStore>) {
        let store = Arc::new(InMemoryCardStore::default());
        let state = Arc::new(CardState {
            config: CardConfig {
                enabled,
                network_passphrase: "Test SDF Network ; September 2015".into(),
            },
            store: store.clone(),
        });
        (crate::card::router_with_state(state), store)
    }

    #[tokio::test]
    async fn test_usd_draft_returns_200() {
        let (app, _) = test_app(true);
        let body = json!({
            "name": "Jane Doe",
            "country": "US",
            "currency": "USD",
            "monthly_limit_usdc": 1000.0
        });

        let (status, resp) = send(
            app,
            "/api/v1/card/applications/validate",
            &[],
            body.to_string(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(resp["name"], "Jane Doe");
        assert_eq!(resp["country"], "US");
        assert_eq!(resp["currency"], "USD");
        assert_eq!(resp["monthly_limit_usdc"], 1000.0);
    }

    #[tokio::test]
    async fn test_eur_and_gbp_drafts_return_200() {
        let (app, _) = test_app(true);

        for (curr, country) in [("eur", "de"), ("GBP", "GB")] {
            let body = json!({
                "name": "Alex Smith",
                "country": country,
                "currency": curr,
                "monthly_limit_usdc": 500
            });

            let (status, resp) = send(
                app.clone(),
                "/api/v1/card/applications/validate",
                &[],
                body.to_string(),
            )
            .await;

            assert_eq!(status, StatusCode::OK);
            assert_eq!(resp["currency"], curr.to_uppercase());
            assert_eq!(resp["country"], country.to_uppercase());
        }
    }

    #[tokio::test]
    async fn test_xxx_currency_returns_422() {
        let (app, _) = test_app(true);
        let body = json!({
            "name": "Jane Doe",
            "country": "US",
            "currency": "XXX",
            "monthly_limit_usdc": 1000.0
        });

        let (status, resp) = send(
            app,
            "/api/v1/card/applications/validate",
            &[],
            body.to_string(),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(resp["error"], "unsupported_currency");
    }

    #[tokio::test]
    async fn test_flag_off_returns_404() {
        let (app, _) = test_app(false);
        let body = json!({
            "name": "Jane Doe",
            "country": "US",
            "currency": "USD",
            "monthly_limit_usdc": 1000.0
        });

        let (status, _) = send(
            app,
            "/api/v1/card/applications/validate",
            &[],
            body.to_string(),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_zero_or_negative_limit_returns_422() {
        let (app, _) = test_app(true);

        for invalid_limit in [0.0, -100.0] {
            let body = json!({
                "name": "Jane Doe",
                "country": "US",
                "currency": "USD",
                "monthly_limit_usdc": invalid_limit
            });

            let (status, resp) = send(
                app.clone(),
                "/api/v1/card/applications/validate",
                &[],
                body.to_string(),
            )
            .await;

            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(resp["error"], "invalid_limit");
        }
    }

    #[tokio::test]
    async fn test_no_persistence_during_validate() {
        let (app, store) = test_app(true);
        let body = json!({
            "name": "Jane Doe",
            "country": "US",
            "currency": "USD",
            "monthly_limit_usdc": 1000.0
        });

        let (status, _) = send(
            app,
            "/api/v1/card/applications/validate",
            &[],
            body.to_string(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        // Verify store has 0 applications
        assert_eq!(store.get_application("Jane Doe"), None);
    }
}
