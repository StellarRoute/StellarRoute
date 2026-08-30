//! Authoritative pricing for swap prepare.
//!
//! Expected output MUST come from a server price source (live quote pipeline or
//! an injected test double) — never from client hop `price`/`fee_bps` fields.

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::error::{ApiError, Result};
use crate::models::request::{AssetPath, QuoteParams, QuoteType};
use crate::routes::quote::get_quote_for_pair_dry_run;
use crate::state::AppState;
use crate::swap::route::ValidatedClassicRoute;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AuthoritativePrice {
    pub expected_output: f64,
    pub price_digest: String,
    pub source_label: String,
}

/// Pluggable price source so prepare never trusts client hop economics.
#[async_trait]
pub trait SwapPriceSource: Send + Sync {
    async fn price_swap(
        &self,
        route: &ValidatedClassicRoute,
        amount_in: f64,
    ) -> Result<AuthoritativePrice>;
}

/// Production source: reprice via the existing quote pipeline / normalized liquidity.
pub struct LiveQuotePriceSource {
    pub state: Arc<AppState>,
}

#[async_trait]
impl SwapPriceSource for LiveQuotePriceSource {
    async fn price_swap(
        &self,
        route: &ValidatedClassicRoute,
        amount_in: f64,
    ) -> Result<AuthoritativePrice> {
        // Sell `send_asset` for `dest_asset` (sell quote).
        let params = QuoteParams {
            amount: Some(format!("{amount_in:.7}")),
            slippage_bps: None,
            quote_type: QuoteType::Sell,
            explain: None,
            fields: None,
        };
        let quote = get_quote_for_pair_dry_run(
            self.state.clone(),
            route.send_asset.clone(),
            route.dest_asset.clone(),
            params,
        )
        .await?;

        // Reject if the live quote selected a non-SDEX venue.
        if let Some(r) = &quote.rationale {
            let src = r.selected_source.to_ascii_lowercase();
            let head = src.split(':').next().unwrap_or(&src);
            if head != "sdex" && head != "horizon" {
                return Err(ApiError::UnsupportedExecutionMode(format!(
                    "live quote selected unsupported venue '{}'; classic SDEX only",
                    r.selected_source
                )));
            }
        }

        let expected: f64 = quote
            .total
            .parse()
            .map_err(|_| ApiError::Internal(Arc::new(anyhow::anyhow!("invalid quote total"))))?;
        if !expected.is_finite() || expected <= 0.0 {
            return Err(ApiError::NoRouteFound);
        }

        let digest = price_digest(
            &route.send_asset,
            &route.dest_asset,
            amount_in,
            expected,
            &route.route_digest,
            quote.timestamp,
        );
        Ok(AuthoritativePrice {
            expected_output: expected,
            price_digest: digest,
            source_label: "live_quote".into(),
        })
    }
}

/// Fixed price for unit/integration tests (never used in production wiring).
#[derive(Debug, Clone)]
pub struct FixedPriceSource {
    pub expected_output_per_unit: f64,
}

#[async_trait]
impl SwapPriceSource for FixedPriceSource {
    async fn price_swap(
        &self,
        route: &ValidatedClassicRoute,
        amount_in: f64,
    ) -> Result<AuthoritativePrice> {
        let expected = amount_in * self.expected_output_per_unit;
        let digest = price_digest(
            &route.send_asset,
            &route.dest_asset,
            amount_in,
            expected,
            &route.route_digest,
            0,
        );
        Ok(AuthoritativePrice {
            expected_output: expected,
            price_digest: digest,
            source_label: "test_fixed".into(),
        })
    }
}

pub fn price_digest(
    send: &AssetPath,
    dest: &AssetPath,
    amount_in: f64,
    expected: f64,
    route_digest: &str,
    quote_ts: i64,
) -> String {
    let mut h = Sha256::new();
    h.update(send.to_canonical().as_bytes());
    h.update(b">");
    h.update(dest.to_canonical().as_bytes());
    h.update(format!("|{amount_in:.7}|{expected:.7}|{route_digest}|{quote_ts}").as_bytes());
    hex::encode(h.finalize())
}

/// Server slippage floor. Client `min_output` may not undercut it; may not exceed expected.
pub fn resolve_min_output(
    expected_output: f64,
    slippage_bps: u32,
    client_min: Option<f64>,
) -> std::result::Result<f64, ApiError> {
    if !expected_output.is_finite() || expected_output <= 0.0 {
        return Err(ApiError::Validation(
            "expected_output must be positive".into(),
        ));
    }
    let floor = expected_output * (1.0 - (slippage_bps as f64 / 10_000.0));
    match client_min {
        Some(m) if !m.is_finite() || m <= 0.0 => Err(ApiError::Validation(
            "min_output must be greater than zero".into(),
        )),
        Some(m) if m > expected_output => Err(ApiError::NotExecutable(
            "min_output exceeds estimated route output".into(),
        )),
        Some(m) if m + f64::EPSILON < floor => Err(ApiError::Validation(format!(
            "min_output {m} is below server slippage floor {floor:.7}"
        ))),
        Some(m) => Ok(m),
        None => Ok(floor),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slippage_floor_rejects_client_undercut() {
        let err = resolve_min_output(100.0, 50, Some(90.0)).unwrap_err();
        assert!(matches!(err, ApiError::Validation(_)));
    }

    #[test]
    fn slippage_floor_accepts_at_boundary() {
        let floor = 100.0 * (1.0 - 50.0 / 10_000.0);
        let v = resolve_min_output(100.0, 50, Some(floor)).unwrap();
        assert!((v - floor).abs() < 1e-9);
    }

    #[test]
    fn rejects_min_above_expected() {
        assert!(matches!(
            resolve_min_output(100.0, 50, Some(101.0)),
            Err(ApiError::NotExecutable(_))
        ));
    }

    #[test]
    fn defaults_to_floor_when_client_omits() {
        let v = resolve_min_output(100.0, 100, None).unwrap();
        assert!((v - 99.0).abs() < 1e-9);
    }
}
