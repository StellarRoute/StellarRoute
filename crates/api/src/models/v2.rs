//! `/api/v2` chain-aware request/response models.
//!
//! Additive seam for future cross-chain quotes. Does not alter `/api/v1`
//! contracts. Asset identities use CAIP-style canonical strings.
//!
//! Bridge edges are **metadata-only / non-executable**. Compaction preserves
//! bridge identity (`venue_type`, provider, bridge meta) so they cannot be
//! laundered into SDEX/AMM. Provider kill-switches apply only to
//! **provider-tagged** candidates — current Stellar `normalized_liquidity`
//! rows have no provider column, so quote selection is unaffected until ingest
//! supplies that metadata.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::v2_cctp::SupportedCorridor;

/// Chain-scoped asset as returned by v2 endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ChainAssetV2 {
    /// CAIP-2 chain id, e.g. `stellar:pubnet`, `eip155:1`.
    pub chain_id: String,
    /// Asset suffix, e.g. `slip44:148`, `erc20:0x…` (numeric SLIP-44 for natives).
    pub asset: String,
    /// Full canonical CAIP-19 identifier (`{chain_id}/{asset}`).
    pub canonical: String,
    /// Optional human symbol (not unique across chains).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// Request body / query for canonicalizing an asset identifier.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CanonicalizeAssetRequest {
    /// Legacy Stellar id (`native`, `CODE:ISSUER`) or CAIP-19 string.
    pub asset: String,
}

/// Response from `POST /api/v2/assets/canonicalize`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CanonicalizeAssetResponse {
    pub asset: ChainAssetV2,
    /// `legacy_stellar` or `caip19`.
    pub input_form: String,
}

/// Lightweight v2 capability descriptor (documents the seam without full quote port).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiV2Info {
    pub version: u8,
    pub chain_aware_assets: bool,
    /// Typed bridge edges are metadata-only; compaction preserves their identity.
    pub bridge_venues_metadata_only: bool,
    /// Always false: bridge settlement is not executable; default pathfinding rejects bridges.
    pub bridge_settlement_executable: bool,
    pub supported_chain_namespaces: Vec<String>,
    /// Advertised CCTP corridors (empty until backend health gates execution).
    pub supported_corridors: Vec<SupportedCorridor>,
}
