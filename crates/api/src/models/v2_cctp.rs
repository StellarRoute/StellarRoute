//! Circle CCTP v2 bridge wire models (contract freeze — no backend execution).
//!
//! Discriminated, snake_case JSON shapes for the first testnet corridor
//! (Stellar testnet domain 27 <-> Ethereum Sepolia domain 0). Handlers remain
//! fail-closed until a later implementation phase wires protocol execution.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::v2::ChainAssetV2;

/// Provider identifier for Circle CCTP v2.
pub const CCTP_PROVIDER_ID: &str = "circle-cctp";

/// Documented testnet corridor id (metadata only; not executable on this branch).
pub const CCTP_TESTNET_CORRIDOR_ID: &str = "circle-cctp:usdc:stellar-testnet:ethereum-sepolia";

pub const STELLAR_TESTNET_CHAIN_ID: &str = "stellar:testnet";
pub const SEPOLIA_CHAIN_ID: &str = "eip155:11155111";

pub const STELLAR_TESTNET_USDC_ASSET: &str =
    "erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA";
pub const STELLAR_TESTNET_USDC_CANONICAL: &str =
    "stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA";

pub const SEPOLIA_USDC_ASSET: &str = "erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238";
pub const SEPOLIA_USDC_CANONICAL: &str =
    "eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238";

/// Strict chain-scoped asset wire shape for CCTP requests (denies unknown fields).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpChainAsset {
    pub chain_id: String,
    pub asset: String,
    pub canonical: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

impl CctpChainAsset {
    pub fn stellar_testnet_usdc() -> Self {
        Self {
            chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
            asset: STELLAR_TESTNET_USDC_ASSET.into(),
            canonical: STELLAR_TESTNET_USDC_CANONICAL.into(),
            symbol: Some("USDC".into()),
        }
    }

    pub fn sepolia_usdc() -> Self {
        Self {
            chain_id: SEPOLIA_CHAIN_ID.into(),
            asset: SEPOLIA_USDC_ASSET.into(),
            canonical: SEPOLIA_USDC_CANONICAL.into(),
            symbol: Some("USDC".into()),
        }
    }

    pub fn to_chain_asset_v2(&self) -> ChainAssetV2 {
        ChainAssetV2 {
            chain_id: self.chain_id.clone(),
            asset: self.asset.clone(),
            canonical: self.canonical.clone(),
            symbol: self.symbol.clone(),
        }
    }
}

/// Bridge transfer direction for the Stellar <-> EVM corridor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CctpDirection {
    StellarToEvm,
    EvmToStellar,
}

/// CCTP finality mode (`standard` = 2000, `fast` = 1000). Both corridor directions
/// may request Fast; Iris prices the fee tier per domain pair.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CctpFinality {
    Standard,
    Fast,
}

/// Saga lifecycle for a CCTP transfer (distinct from HTTP error codes).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CctpTransferStatus {
    Created,
    BurnPrepared,
    BurnSubmitted,
    AwaitingAttestation,
    AttestationReady,
    MintPrepared,
    MintSubmitted,
    Completed,
    AttestationFailed,
    MintFailedRetryable,
    Cancelled,
    ProviderKilled,
}

/// Advertised corridor capability (empty by default until backend health gates execution).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SupportedCorridor {
    pub corridor_id: String,
    pub provider: String,
    pub direction: CctpDirection,
    pub source_chain_id: String,
    pub destination_chain_id: String,
    pub source_asset: CctpChainAsset,
    pub destination_asset: CctpChainAsset,
    /// Always false on the contract-freeze branch.
    pub executable: bool,
}

/// Runtime fee quote fields — no invented fixed fees.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpFeeQuote {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_fee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_fee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_fee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_asset: Option<CctpChainAsset>,
}

/// Prepared wallet payload union returned by prepare-burn / prepare-mint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PreparedWalletPayload {
    StellarXdr {
        network_passphrase: String,
        xdr_envelope: String,
        /// Optional signing account (G) — set for trustline ChangeTrust payloads.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
    EvmTransaction {
        chain_id: String,
        to: String,
        data: String,
        value: String,
    },
}

/// Typed status/error details on transfer polling responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpStatusDetails {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
}

/// `POST /api/v2/bridge/cctp/quote` request body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpQuoteRequest {
    pub corridor_id: String,
    pub provider: String,
    pub direction: CctpDirection,
    pub source_chain_id: String,
    pub destination_chain_id: String,
    pub source_asset: CctpChainAsset,
    pub destination_asset: CctpChainAsset,
    /// Decimal string amount (never float).
    pub amount: String,
    /// Destination recipient: EVM `0x` address for `stellar_to_evm`, or Stellar G/M strkey for `evm_to_stellar`.
    pub recipient: String,
    /// Optional source sender: Stellar G-address when burning from Stellar, EVM address when burning from EVM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    /// Stellar G-address fee-payer/submitter for `evm_to_stellar` mint preparation (distinct from `recipient`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint_submitter: Option<String>,
    pub finality: CctpFinality,
}

/// Validation failures surfaced before fail-closed `cctp_not_enabled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CctpValidationError {
    UnsupportedCorridor,
    InvalidFinality,
    InvalidRecipient,
    InvalidAmount,
    InvalidSender,
    InvalidMintSubmitter,
    StellarRemainder,
}

impl CctpQuoteRequest {
    /// Contract validation shared by handlers and unit tests.
    pub fn validate(&self) -> Result<(), CctpValidationError> {
        if self.corridor_id != CCTP_TESTNET_CORRIDOR_ID || self.provider != CCTP_PROVIDER_ID {
            return Err(CctpValidationError::UnsupportedCorridor);
        }

        let (
            expected_source_chain,
            expected_dest_chain,
            expected_source_asset,
            expected_source_canonical,
            expected_dest_asset,
            expected_dest_canonical,
        ) = match self.direction {
            CctpDirection::StellarToEvm => (
                STELLAR_TESTNET_CHAIN_ID,
                SEPOLIA_CHAIN_ID,
                STELLAR_TESTNET_USDC_ASSET,
                STELLAR_TESTNET_USDC_CANONICAL,
                SEPOLIA_USDC_ASSET,
                SEPOLIA_USDC_CANONICAL,
            ),
            CctpDirection::EvmToStellar => (
                SEPOLIA_CHAIN_ID,
                STELLAR_TESTNET_CHAIN_ID,
                SEPOLIA_USDC_ASSET,
                SEPOLIA_USDC_CANONICAL,
                STELLAR_TESTNET_USDC_ASSET,
                STELLAR_TESTNET_USDC_CANONICAL,
            ),
        };

        if self.source_chain_id != expected_source_chain
            || self.destination_chain_id != expected_dest_chain
            || !asset_matches_frozen(
                &self.source_asset,
                expected_source_chain,
                expected_source_asset,
                expected_source_canonical,
            )
            || !asset_matches_frozen(
                &self.destination_asset,
                expected_dest_chain,
                expected_dest_asset,
                expected_dest_canonical,
            )
        {
            return Err(CctpValidationError::UnsupportedCorridor);
        }

        if !is_valid_positive_decimal_amount(&self.amount) {
            return Err(CctpValidationError::InvalidAmount);
        }

        match self.direction {
            CctpDirection::StellarToEvm => {
                if !is_valid_evm_address(&self.recipient) {
                    return Err(CctpValidationError::InvalidRecipient);
                }
                let sender = self
                    .sender
                    .as_deref()
                    .ok_or(CctpValidationError::InvalidSender)?;
                if !is_valid_stellar_account(sender) {
                    return Err(CctpValidationError::InvalidSender);
                }
                // Fast allowed for Stellar→EVM (Iris prices threshold 1000 for 27→0).
            }
            CctpDirection::EvmToStellar => {
                if !is_valid_stellar_recipient(&self.recipient) {
                    return Err(CctpValidationError::InvalidRecipient);
                }
                let sender = self
                    .sender
                    .as_deref()
                    .ok_or(CctpValidationError::InvalidSender)?;
                if !is_valid_evm_address(sender) {
                    return Err(CctpValidationError::InvalidSender);
                }
                let submitter = self
                    .mint_submitter
                    .as_deref()
                    .ok_or(CctpValidationError::InvalidMintSubmitter)?;
                if !is_valid_stellar_account(submitter) {
                    return Err(CctpValidationError::InvalidMintSubmitter);
                }
                // Fast is allowed for EVM→Stellar (Sepolia Iris prices threshold 1000).
            }
        }

        Ok(())
    }
}

fn asset_matches_frozen(
    asset: &CctpChainAsset,
    chain_id: &str,
    asset_id: &str,
    canonical: &str,
) -> bool {
    asset.chain_id == chain_id && asset.asset == asset_id && asset.canonical == canonical
}

/// Positive decimal string: digits with optional single `.` fraction; no sign/exponent/whitespace.
pub fn is_valid_positive_decimal_amount(amount: &str) -> bool {
    if amount.is_empty() || amount.contains([' ', '\t', '\n', '\r', '+', '-', 'e', 'E']) {
        return false;
    }
    let mut parts = amount.split('.');
    let whole = parts.next().unwrap_or("");
    if whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if whole.len() > 1 && whole.starts_with('0') {
        return false;
    }
    match parts.next() {
        None => whole != "0",
        Some(fraction) => {
            if parts.next().is_some()
                || fraction.is_empty()
                || !fraction.chars().all(|c| c.is_ascii_digit())
            {
                return false;
            }
            !(whole == "0" && fraction.chars().all(|c| c == '0'))
        }
    }
}

pub fn is_valid_evm_address(address: &str) -> bool {
    let trimmed = address.trim();
    if trimmed.len() != 42 || !trimmed.starts_with("0x") {
        return false;
    }
    trimmed[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Stellar account recipient (G- or M-address; contract C-strkeys rejected).
pub fn is_valid_stellar_recipient(address: &str) -> bool {
    crate::cctp::stellar_muxed::parse_recipient_strkey(address)
        .ok()
        .is_some_and(|k| {
            matches!(
                k,
                crate::cctp::stellar_muxed::StellarRecipientKey::Account(_)
                    | crate::cctp::stellar_muxed::StellarRecipientKey::Muxed { .. }
            )
        })
}

/// Stellar account recipient (G-address only; muxed M-addresses are not accepted).
pub fn is_valid_stellar_account(address: &str) -> bool {
    stellar_strkey::ed25519::PublicKey::from_string(address.trim()).is_ok()
}

/// Parse a transfer id path parameter (UUID v4 wire form).
pub fn parse_transfer_id(transfer_id: &str) -> Result<Uuid, String> {
    Uuid::parse_str(transfer_id).map_err(|_| format!("Invalid transfer ID: {transfer_id}"))
}

/// Accept Stellar (64-hex) or EVM (`0x` + 64-hex) transaction hash acknowledgement forms.
pub fn is_valid_tx_hash(tx_hash: &str) -> bool {
    let trimmed = tx_hash.trim();
    if trimmed.is_empty() {
        return false;
    }
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    trimmed.len() == 64 && trimmed.chars().all(|c| c.is_ascii_hexdigit())
}

/// `POST /api/v2/bridge/cctp/quote` success response (not returned until enabled).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpQuoteResponse {
    pub transfer_id: String,
    pub corridor_id: String,
    pub provider: String,
    pub direction: CctpDirection,
    pub source_amount: String,
    pub destination_amount: String,
    pub fee_quote: CctpFeeQuote,
    pub expires_at: i64,
    pub finality: CctpFinality,
    /// One-time bearer capability for transfer mutations/status (returned only at quote creation).
    pub access_token: String,
}

/// `GET /api/v2/bridge/cctp/{transfer_id}` response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpTransferStatusResponse {
    pub transfer_id: String,
    pub corridor_id: String,
    pub provider: String,
    pub direction: CctpDirection,
    pub status: CctpTransferStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_reference_id: Option<String>,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CctpStatusDetails>,
    /// Unix seconds (UTC) until re-attest may be requested again; safe for UI countdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reattest_cooldown_until: Option<i64>,
}

/// `POST /api/v2/bridge/cctp/{transfer_id}/prepare-burn` response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpPrepareBurnResponse {
    pub transfer_id: String,
    pub status: CctpTransferStatus,
    pub payload: PreparedWalletPayload,
    pub expires_at: i64,
    /// When true, wallet must submit approval tx and call record-approval before requesting burn payload.
    #[serde(default)]
    pub approval_required: bool,
}

/// Burn submit accepts only an on-chain tx hash acknowledgement.
///
/// Signed transaction broadcasting is the wallet/provider responsibility;
/// the API records the hash for attestation polling and later verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpSubmitBurnRequest {
    pub tx_hash: String,
}

/// `POST /api/v2/bridge/cctp/{transfer_id}/submit-burn` response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpSubmitBurnResponse {
    pub transfer_id: String,
    pub status: CctpTransferStatus,
    pub source_tx_hash: String,
}

/// `POST /api/v2/bridge/cctp/{transfer_id}/prepare-mint` response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpPrepareMintResponse {
    pub transfer_id: String,
    pub status: CctpTransferStatus,
    pub payload: PreparedWalletPayload,
    pub expires_at: i64,
    /// True when the wallet must submit a USDC ChangeTrust before `mint_and_forward`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub trustline_required: bool,
}

/// Mint submit accepts only an on-chain tx hash acknowledgement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpSubmitMintRequest {
    pub tx_hash: String,
}

/// `POST /api/v2/bridge/cctp/{transfer_id}/submit-mint` response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpSubmitMintResponse {
    pub transfer_id: String,
    pub status: CctpTransferStatus,
    pub destination_tx_hash: String,
}

/// `POST /api/v2/bridge/cctp/{transfer_id}/reattest` response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CctpReattestResponse {
    pub transfer_id: String,
    pub status: CctpTransferStatus,
    pub retryable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_EVM: &str = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
    const VALID_STELLAR: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
    const VALID_EVM_TX: &str = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    const VALID_STELLAR_TX: &str =
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

    fn base_quote(direction: CctpDirection, finality: CctpFinality) -> CctpQuoteRequest {
        let (source_chain, dest_chain, source_asset, dest_asset, recipient) = match direction {
            CctpDirection::StellarToEvm => (
                STELLAR_TESTNET_CHAIN_ID,
                SEPOLIA_CHAIN_ID,
                CctpChainAsset::stellar_testnet_usdc(),
                CctpChainAsset::sepolia_usdc(),
                VALID_EVM.to_string(),
            ),
            CctpDirection::EvmToStellar => (
                SEPOLIA_CHAIN_ID,
                STELLAR_TESTNET_CHAIN_ID,
                CctpChainAsset::sepolia_usdc(),
                CctpChainAsset::stellar_testnet_usdc(),
                VALID_STELLAR.to_string(),
            ),
        };

        let mint_submitter = match direction {
            CctpDirection::EvmToStellar => Some(VALID_STELLAR.to_string()),
            CctpDirection::StellarToEvm => None,
        };

        CctpQuoteRequest {
            corridor_id: CCTP_TESTNET_CORRIDOR_ID.into(),
            provider: CCTP_PROVIDER_ID.into(),
            direction,
            source_chain_id: source_chain.into(),
            destination_chain_id: dest_chain.into(),
            source_asset,
            destination_asset: dest_asset,
            amount: "10.0".into(),
            recipient,
            sender: None,
            mint_submitter,
            finality,
        }
    }

    #[test]
    fn accepts_stellar_source_fast_finality() {
        let mut req = base_quote(CctpDirection::StellarToEvm, CctpFinality::Fast);
        req.sender = Some(VALID_STELLAR.to_string());
        assert_eq!(req.validate(), Ok(()));
    }

    #[test]
    fn accepts_evm_source_fast_finality() {
        let mut req = base_quote(CctpDirection::EvmToStellar, CctpFinality::Fast);
        req.sender = Some(VALID_EVM.to_string());
        assert_eq!(req.validate(), Ok(()));
    }

    #[test]
    fn rejects_unknown_top_level_fields_on_quote_request() {
        let json = r#"{
            "corridor_id":"circle-cctp:usdc:stellar-testnet:ethereum-sepolia",
            "provider":"circle-cctp","direction":"stellar_to_evm",
            "source_chain_id":"stellar:testnet","destination_chain_id":"eip155:11155111",
            "source_asset":{"chain_id":"stellar:testnet","asset":"erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA","canonical":"stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA"},
            "destination_asset":{"chain_id":"eip155:11155111","asset":"erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238","canonical":"eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"},
            "amount":"1","recipient":"0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0","finality":"standard","extra":true
        }"#;
        assert!(serde_json::from_str::<CctpQuoteRequest>(json).is_err());
    }

    #[test]
    fn rejects_unknown_nested_asset_fields() {
        let json = r#"{
            "corridor_id":"circle-cctp:usdc:stellar-testnet:ethereum-sepolia",
            "provider":"circle-cctp","direction":"stellar_to_evm",
            "source_chain_id":"stellar:testnet","destination_chain_id":"eip155:11155111",
            "source_asset":{"chain_id":"stellar:testnet","asset":"erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA","canonical":"stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA","extra":1},
            "destination_asset":{"chain_id":"eip155:11155111","asset":"erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238","canonical":"eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"},
            "amount":"1","recipient":"0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0","finality":"standard"
        }"#;
        assert!(serde_json::from_str::<CctpQuoteRequest>(json).is_err());
    }

    #[test]
    fn rejects_inconsistent_asset_and_canonical() {
        let mut req = base_quote(CctpDirection::StellarToEvm, CctpFinality::Standard);
        req.source_asset.asset = SEPOLIA_USDC_ASSET.into();
        assert_eq!(
            req.validate(),
            Err(CctpValidationError::UnsupportedCorridor)
        );

        let mut req2 = base_quote(CctpDirection::EvmToStellar, CctpFinality::Standard);
        req2.source_asset.canonical = STELLAR_TESTNET_USDC_CANONICAL.into();
        assert_eq!(
            req2.validate(),
            Err(CctpValidationError::UnsupportedCorridor)
        );
    }

    #[test]
    fn rejects_wrong_corridor_id() {
        let mut req = base_quote(CctpDirection::StellarToEvm, CctpFinality::Standard);
        req.corridor_id = "other".into();
        assert_eq!(
            req.validate(),
            Err(CctpValidationError::UnsupportedCorridor)
        );
    }

    #[test]
    fn rejects_wrong_provider() {
        let mut req = base_quote(CctpDirection::StellarToEvm, CctpFinality::Standard);
        req.provider = "other".into();
        assert_eq!(
            req.validate(),
            Err(CctpValidationError::UnsupportedCorridor)
        );
    }

    #[test]
    fn rejects_mismatched_direction_and_chains() {
        let mut req = base_quote(CctpDirection::StellarToEvm, CctpFinality::Standard);
        req.source_chain_id = SEPOLIA_CHAIN_ID.into();
        assert_eq!(
            req.validate(),
            Err(CctpValidationError::UnsupportedCorridor)
        );
    }

    #[test]
    fn rejects_invalid_amounts() {
        for amount in ["", " ", "0", "0.0", "-1", "1e3", "1.2.3", "00.5", "1."] {
            assert!(
                !is_valid_positive_decimal_amount(amount),
                "expected invalid: {amount}"
            );
        }
        assert!(is_valid_positive_decimal_amount("100.000000"));
        assert!(is_valid_positive_decimal_amount("0.5"));
    }

    #[test]
    fn rejects_invalid_recipient_per_direction() {
        let mut to_evm = base_quote(CctpDirection::StellarToEvm, CctpFinality::Standard);
        to_evm.recipient = VALID_STELLAR.into();
        assert_eq!(
            to_evm.validate(),
            Err(CctpValidationError::InvalidRecipient)
        );

        let mut to_stellar = base_quote(CctpDirection::EvmToStellar, CctpFinality::Standard);
        to_stellar.recipient = VALID_EVM.into();
        assert_eq!(
            to_stellar.validate(),
            Err(CctpValidationError::InvalidRecipient)
        );
    }

    #[test]
    fn requires_valid_sender_per_direction() {
        let mut to_evm = base_quote(CctpDirection::StellarToEvm, CctpFinality::Standard);
        to_evm.sender = None;
        assert_eq!(to_evm.validate(), Err(CctpValidationError::InvalidSender));
        to_evm.sender = Some(VALID_EVM.into());
        assert_eq!(to_evm.validate(), Err(CctpValidationError::InvalidSender));
        to_evm.sender = Some(VALID_STELLAR.into());
        assert!(to_evm.validate().is_ok());

        let mut to_stellar = base_quote(CctpDirection::EvmToStellar, CctpFinality::Standard);
        to_stellar.sender = None;
        assert_eq!(
            to_stellar.validate(),
            Err(CctpValidationError::InvalidSender)
        );
        to_stellar.sender = Some(VALID_STELLAR.into());
        assert_eq!(
            to_stellar.validate(),
            Err(CctpValidationError::InvalidSender)
        );
        to_stellar.sender = Some(VALID_EVM.into());
        assert!(to_stellar.validate().is_ok());
    }

    #[test]
    fn tx_hash_validation_accepts_stellar_and_evm_forms() {
        assert!(is_valid_tx_hash(VALID_EVM_TX));
        assert!(is_valid_tx_hash(VALID_STELLAR_TX));
        assert!(!is_valid_tx_hash(""));
        assert!(!is_valid_tx_hash("0xabc"));
        assert!(!is_valid_tx_hash("not-a-hash"));
    }

    #[test]
    fn parse_transfer_id_requires_uuid() {
        assert!(parse_transfer_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(parse_transfer_id("not-a-uuid").is_err());
    }
}
