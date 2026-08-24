//! Typed compatibility layer between `/api/v1` Stellar asset models and
//! chain-scoped identifiers used by the v2 seam / routing foundation.

use super::response::AssetInfo;
use super::v2::ChainAssetV2;
use stellarroute_routing::chain_asset::{looks_like_caip, AssetReference, ChainAsset, ChainId};
use stellarroute_routing::error::RoutingError;

/// Convert a v1 [`AssetInfo`] into a chain-scoped asset (Stellar pubnet).
pub fn asset_info_to_chain_asset(info: &AssetInfo) -> Result<ChainAsset, RoutingError> {
    match info.asset_type.as_str() {
        "native" => Ok(ChainAsset::stellar_native("pubnet")),
        _ => match (&info.asset_code, &info.asset_issuer) {
            (Some(code), Some(issuer)) => ChainAsset::stellar_credit("pubnet", code, issuer),
            (Some(code), None) => Err(RoutingError::InvalidAsset(format!(
                "issued asset {code} requires issuer for chain-scoped form"
            ))),
            _ => Err(RoutingError::InvalidAsset(
                "invalid AssetInfo for chain conversion".to_string(),
            )),
        },
    }
}

/// Convert a chain-scoped Stellar asset back to v1 [`AssetInfo`].
///
/// Non-Stellar assets cannot be represented in the v1 model.
pub fn chain_asset_to_asset_info(asset: &ChainAsset) -> Result<AssetInfo, RoutingError> {
    match (&asset.chain, &asset.asset) {
        (ChainId::Stellar { .. }, AssetReference::Native) => Ok(AssetInfo::native()),
        (ChainId::Stellar { .. }, AssetReference::StellarCredit { code, issuer }) => {
            Ok(AssetInfo::credit(code.clone(), Some(issuer.clone())))
        }
        _ => Err(RoutingError::InvalidAsset(format!(
            "cannot project non-Stellar asset into v1 AssetInfo: {}",
            asset.to_canonical()
        ))),
    }
}

/// Build a v2 response DTO from a [`ChainAsset`].
pub fn chain_asset_to_v2(asset: &ChainAsset) -> ChainAssetV2 {
    let symbol = match &asset.asset {
        AssetReference::Native => Some(match &asset.chain {
            ChainId::Stellar { .. } => "XLM".to_string(),
            ChainId::Eip155 { .. } => "ETH".to_string(),
            ChainId::Solana { .. } => "SOL".to_string(),
            ChainId::Bitcoin { .. } => "BTC".to_string(),
            ChainId::Tron { .. } => "TRX".to_string(),
        }),
        AssetReference::StellarCredit { code, .. } => Some(code.clone()),
        AssetReference::Erc20 { .. }
        | AssetReference::SplToken { .. }
        | AssetReference::Trc20 { .. } => None,
    };

    let canonical = asset.to_canonical();
    let (chain_id, asset_suffix) = canonical
        .split_once('/')
        .map(|(c, a)| (c.to_string(), a.to_string()))
        .unwrap_or_else(|| {
            (
                asset.chain.to_caip2(),
                asset.asset.to_caip19_suffix(&asset.chain),
            )
        });

    ChainAssetV2 {
        chain_id,
        asset: asset_suffix,
        canonical,
        symbol,
    }
}

/// Parse either a legacy Stellar id or chain-scoped id into [`ChainAsset`].
pub fn parse_asset_input(input: &str) -> Result<(ChainAsset, &'static str), RoutingError> {
    if looks_like_caip(input) {
        Ok((ChainAsset::parse(input)?, "caip19"))
    } else {
        Ok((ChainAsset::from_stellar_legacy(input)?, "legacy_stellar"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_native_round_trip() {
        let info = AssetInfo::native();
        let chain = asset_info_to_chain_asset(&info).unwrap();
        let back = chain_asset_to_asset_info(&chain).unwrap();
        assert_eq!(back.asset_type, "native");
        assert_eq!(chain.to_canonical(), "stellar:pubnet/slip44:148");
    }

    #[test]
    fn ethereum_asset_rejected_for_v1_projection() {
        let eth = ChainAsset::new(ChainId::ethereum_mainnet(), AssetReference::Native).unwrap();
        assert!(chain_asset_to_asset_info(&eth).is_err());
    }

    #[test]
    fn parse_legacy_and_caip() {
        let (a, form) = parse_asset_input("XLM").unwrap();
        assert_eq!(form, "legacy_stellar");
        assert_eq!(a.to_canonical(), "stellar:pubnet/slip44:148");

        let (b, form) = parse_asset_input("eip155:1/slip44:60").unwrap();
        assert_eq!(form, "caip19");
        assert_eq!(b.chain_namespace(), "eip155");
    }

    #[test]
    fn malformed_caip_fails_closed() {
        assert!(parse_asset_input("eip155:1/slip44:native").is_err());
        assert!(parse_asset_input("stellar:pubnet/native").is_err());
    }
}
