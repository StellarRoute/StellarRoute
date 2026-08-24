//! Classic-only route validation shared by prepare and the XDR builder.

use sha2::{Digest, Sha256};

use crate::error::ApiError;
use crate::models::request::AssetPath;
use crate::routes::simulation_route::{RouteDryRunHop, RouteDryRunPath};
use crate::swap::venue::{classify_venue, SwapVenueClass};

/// Maximum intermediate path assets for PathPaymentStrictSend (Stellar XDR limit is 5).
pub const MAX_PATH_ASSETS: usize = 5;
/// Production prepare currently accepts a single SDEX hop only.
/// Multi-hop is rejected until authoritative path-faithful pricing exists.
pub const MAX_CLASSIC_HOPS: usize = 1;

/// Validated, normalized classic SDEX route ready for PathPaymentStrictSend.
#[derive(Debug, Clone)]
pub struct ValidatedClassicRoute {
    pub hops: Vec<RouteDryRunHop>,
    pub send_asset: AssetPath,
    pub dest_asset: AssetPath,
    /// Intermediate assets (PathPayment `path` field), excluding send/dest.
    pub path_assets: Vec<AssetPath>,
    pub route_digest: String,
}

/// Validate a prepare route for classic PathPaymentStrictSend only.
pub fn validate_classic_route(route: &RouteDryRunPath) -> Result<ValidatedClassicRoute, ApiError> {
    if route.hops.is_empty() {
        return Err(ApiError::Validation(
            "route.hops must contain at least one hop".to_string(),
        ));
    }
    if route.hops.len() > MAX_CLASSIC_HOPS {
        return Err(ApiError::UnsupportedRoute(format!(
            "multi-hop classic routes are not supported in this prepare build \
             (got {} hops; single SDEX hop only until path-faithful pricing ships)",
            route.hops.len()
        )));
    }

    for (idx, hop) in route.hops.iter().enumerate() {
        validate_asset(&hop.from_asset, &format!("hops[{idx}].from_asset"))?;
        validate_asset(&hop.to_asset, &format!("hops[{idx}].to_asset"))?;

        match classify_venue(&hop.source, hop.venue_ref.as_deref()) {
            SwapVenueClass::Sdex => {}
            SwapVenueClass::AmmOrRouter => {
                return Err(ApiError::UnsupportedExecutionMode(
                    "AMM/Soroban/router venues are not supported; classic PathPaymentStrictSend only"
                        .to_string(),
                ));
            }
            SwapVenueClass::Unknown => {
                return Err(ApiError::Validation(format!(
                    "unsupported classic venue '{}'; expected sdex or horizon",
                    hop.source
                )));
            }
        }
    }

    // Contiguity: hop[i].to == hop[i+1].from
    for i in 1..route.hops.len() {
        let prev = &route.hops[i - 1];
        let curr = &route.hops[i];
        if !assets_equal(&prev.to_asset, &curr.from_asset) {
            return Err(ApiError::Validation(format!(
                "route hops are not contiguous: hop[{}].to_asset must match hop[{}].from_asset",
                i - 1,
                i
            )));
        }
    }

    // Cycle detection on asset progression (send asset must not reappear as an intermediate).
    let mut seen = Vec::new();
    seen.push(canonical(&route.hops[0].from_asset));
    for hop in &route.hops {
        let to = canonical(&hop.to_asset);
        if seen.contains(&to) {
            return Err(ApiError::Validation(
                "route contains a cycle in asset progression".to_string(),
            ));
        }
        seen.push(to);
    }

    let send_asset = route.hops[0].from_asset.clone();
    let dest_asset = route.hops.last().unwrap().to_asset.clone();
    let path_assets: Vec<AssetPath> = route
        .hops
        .iter()
        .take(route.hops.len().saturating_sub(1))
        .map(|h| h.to_asset.clone())
        .collect();

    if path_assets.len() > MAX_PATH_ASSETS {
        return Err(ApiError::Validation(format!(
            "path exceeds maximum of {MAX_PATH_ASSETS} intermediate assets"
        )));
    }

    let route_digest = digest_route(route);
    Ok(ValidatedClassicRoute {
        hops: route.hops.clone(),
        send_asset,
        dest_asset,
        path_assets,
        route_digest,
    })
}

fn validate_asset(asset: &AssetPath, label: &str) -> Result<(), ApiError> {
    let code = asset.asset_code.trim();
    if code.is_empty() {
        return Err(ApiError::InvalidAssetFormat(format!(
            "{label}: asset_code is required"
        )));
    }
    if code.eq_ignore_ascii_case("native") || code.eq_ignore_ascii_case("xlm") {
        return Ok(());
    }
    if code.len() > 12 {
        return Err(ApiError::InvalidAssetFormat(format!(
            "{label}: asset_code too long"
        )));
    }
    let issuer = asset.asset_issuer.as_deref().map(str::trim).unwrap_or("");
    if issuer.is_empty() {
        return Err(ApiError::InvalidAsset(format!(
            "{label}: non-native asset requires asset_issuer (CODE:ISSUER)"
        )));
    }
    if stellar_strkey::ed25519::PublicKey::from_string(issuer).is_err() {
        return Err(ApiError::InvalidAsset(format!(
            "{label}: asset_issuer is not a valid G-address"
        )));
    }
    Ok(())
}

fn assets_equal(a: &AssetPath, b: &AssetPath) -> bool {
    canonical(a) == canonical(b)
}

fn canonical(asset: &AssetPath) -> String {
    asset.to_canonical().to_ascii_lowercase()
}

pub fn digest_route(route: &RouteDryRunPath) -> String {
    let mut hasher = Sha256::new();
    for hop in &route.hops {
        hasher.update(canonical(&hop.from_asset).as_bytes());
        hasher.update(b"|");
        hasher.update(canonical(&hop.to_asset).as_bytes());
        hasher.update(b"|");
        hasher.update(
            classify_venue(&hop.source, hop.venue_ref.as_deref())
                .as_str()
                .as_bytes(),
        );
        hasher.update(b"|");
        if let Some(vr) = &hop.venue_ref {
            hasher.update(vr.trim().as_bytes());
        }
        hasher.update(b";");
    }
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISSUER: &str = "GCXKG6RN4ONIEPCMNFB732A436Z5PNDSRLGWK7GBLCMQLIFO4S7EYWVU";

    fn hop(from: &str, to_code: &str, source: &str) -> RouteDryRunHop {
        let from_asset = if from == "native" {
            AssetPath {
                asset_code: "native".into(),
                asset_issuer: None,
            }
        } else {
            AssetPath {
                asset_code: from.into(),
                asset_issuer: Some(ISSUER.into()),
            }
        };
        let to_asset = if to_code == "native" {
            AssetPath {
                asset_code: "native".into(),
                asset_issuer: None,
            }
        } else {
            AssetPath {
                asset_code: to_code.into(),
                asset_issuer: Some(ISSUER.into()),
            }
        };
        RouteDryRunHop {
            from_asset,
            to_asset,
            source: source.into(),
            fee_bps: Some(30),
            price: Some("0.12".into()),
            venue_ref: Some("sdex-venue".into()),
        }
    }

    #[test]
    fn accepts_contiguous_sdex_route() {
        let route = RouteDryRunPath {
            hops: vec![hop("native", "USDC", "sdex")],
        };
        let v = validate_classic_route(&route).unwrap();
        assert_eq!(v.send_asset.asset_code, "native");
        assert_eq!(v.dest_asset.asset_code, "USDC");
        assert!(v.path_assets.is_empty());
    }

    #[test]
    fn rejects_multi_hop_as_unsupported_route() {
        let route = RouteDryRunPath {
            hops: vec![hop("native", "USDC", "sdex"), hop("USDC", "EURC", "sdex")],
        };
        // Contiguity fix for second hop from-asset.
        let mut hops = route.hops;
        hops[1].from_asset = hops[0].to_asset.clone();
        let route = RouteDryRunPath { hops };
        assert!(matches!(
            validate_classic_route(&route),
            Err(ApiError::UnsupportedRoute(_))
        ));
    }

    #[test]
    fn rejects_missing_issuer() {
        let route = RouteDryRunPath {
            hops: vec![RouteDryRunHop {
                from_asset: AssetPath {
                    asset_code: "native".into(),
                    asset_issuer: None,
                },
                to_asset: AssetPath {
                    asset_code: "USDC".into(),
                    asset_issuer: None,
                },
                source: "sdex".into(),
                fee_bps: None,
                price: None,
                venue_ref: None,
            }],
        };
        assert!(matches!(
            validate_classic_route(&route),
            Err(ApiError::InvalidAsset(_))
        ));
    }

    #[test]
    fn rejects_amm_venue_as_unsupported_mode() {
        let route = RouteDryRunPath {
            hops: vec![hop("native", "USDC", "amm")],
        };
        assert!(matches!(
            validate_classic_route(&route),
            Err(ApiError::UnsupportedExecutionMode(_))
        ));
    }

    #[test]
    fn rejects_mixed_sdex_amm_as_multi_hop_unsupported_route() {
        // Multi-hop is rejected before per-hop AMM classification in this build.
        let route = RouteDryRunPath {
            hops: vec![hop("native", "USDC", "sdex"), hop("USDC", "EURC", "amm")],
        };
        let mut hops = route.hops;
        hops[1].from_asset = hops[0].to_asset.clone();
        let route = RouteDryRunPath { hops };
        assert!(matches!(
            validate_classic_route(&route),
            Err(ApiError::UnsupportedRoute(_))
        ));
    }
}
