//! Serialize prepared burn/mint bundles for durable prepare-lock idempotency.

use serde::{Deserialize, Serialize};

use crate::cctp::builders::{BurnPrepareStep, PreparedBurnBundle, PreparedMintBundle};
use crate::cctp::prepare_lock::MAX_PREPARED_PAYLOAD_LEN;
use crate::models::v2_cctp::PreparedWalletPayload;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PreparePayloadCacheError {
    #[error("serialization: {0}")]
    Serialization(String),
    #[error("payload too large")]
    TooLarge,
}

#[derive(Serialize, Deserialize)]
struct BurnBundleCache {
    step: String,
    approval_required: bool,
    primary: PreparedWalletPayload,
    required_approvals: Vec<PreparedWalletPayload>,
    required_prior_payloads: Vec<PreparedWalletPayload>,
    expires_at: i64,
    approval_expiration_ledger: Option<u32>,
}

pub fn serialize_burn_bundle(
    bundle: &PreparedBurnBundle,
) -> Result<String, PreparePayloadCacheError> {
    let step = match bundle.step {
        BurnPrepareStep::Approval => "approval",
        BurnPrepareStep::Burn => "burn",
    };
    let cache = BurnBundleCache {
        step: step.into(),
        approval_required: bundle.approval_required,
        primary: bundle.primary.clone(),
        required_approvals: bundle.required_approvals.clone(),
        required_prior_payloads: bundle.required_prior_payloads.clone(),
        expires_at: bundle.expires_at,
        approval_expiration_ledger: bundle.approval_expiration_ledger,
    };
    let json = serde_json::to_string(&cache)
        .map_err(|e| PreparePayloadCacheError::Serialization(e.to_string()))?;
    if json.len() > MAX_PREPARED_PAYLOAD_LEN {
        return Err(PreparePayloadCacheError::TooLarge);
    }
    Ok(json)
}

pub fn deserialize_burn_bundle(json: &str) -> Result<PreparedBurnBundle, PreparePayloadCacheError> {
    let cache: BurnBundleCache = serde_json::from_str(json)
        .map_err(|e| PreparePayloadCacheError::Serialization(e.to_string()))?;
    let step = match cache.step.as_str() {
        "approval" => BurnPrepareStep::Approval,
        "burn" => BurnPrepareStep::Burn,
        other => {
            return Err(PreparePayloadCacheError::Serialization(format!(
                "unknown burn step: {other}"
            )))
        }
    };
    Ok(PreparedBurnBundle {
        step,
        approval_required: cache.approval_required,
        primary: cache.primary,
        required_approvals: cache.required_approvals,
        required_prior_payloads: cache.required_prior_payloads,
        expires_at: cache.expires_at,
        approval_expiration_ledger: cache.approval_expiration_ledger,
    })
}

pub fn serialize_mint_bundle(
    bundle: &PreparedMintBundle,
) -> Result<String, PreparePayloadCacheError> {
    let json = serde_json::to_string(bundle)
        .map_err(|e| PreparePayloadCacheError::Serialization(e.to_string()))?;
    if json.len() > MAX_PREPARED_PAYLOAD_LEN {
        return Err(PreparePayloadCacheError::TooLarge);
    }
    Ok(json)
}

pub fn deserialize_mint_bundle(json: &str) -> Result<PreparedMintBundle, PreparePayloadCacheError> {
    serde_json::from_str(json).map_err(|e| PreparePayloadCacheError::Serialization(e.to_string()))
}
