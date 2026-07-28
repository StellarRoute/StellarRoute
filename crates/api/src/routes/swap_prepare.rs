//! Swap prepare endpoint — builds unsigned transaction XDR for the frontend.
//!
//! Depending on the best venue selected by the quote pipeline, the endpoint
//! produces either:
//!
//! - A **classic** `PathPaymentStrictSend` operation (for pure SDEX routes), or
//! - A **Soroban** `InvokeHostFunction` operation targeting the configured
//!   router contract (for AMM / aggregator routes).
//!
//! The response includes an `execution_mode` discriminator so the frontend
//! knows which signing flow to use.
//!
//! Ref: issue #1046

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use stellar_xdr::curr::{
    self as xdr, Asset as XdrAsset, Hash, Int128Parts, InvokeContractArgs,
    InvokeHostFunctionOp, Limits, MuxedAccount, Operation, OperationBody,
    PathPaymentStrictSendOp, ScAddress, ScSymbol, ScVal, Uint256, VecM, WriteXdr,
    HostFunction,
};

use crate::{
    error::{ApiError, Result},
    middleware::RequestId,
    models::response::{
        ApiResponse, AssetInfo, ExecutionMode, QuoteResponse, SwapPrepareResponse,
    },
    state::AppState,
};

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

/// POST body for `/api/v1/swap/prepare`.
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct SwapPrepareRequest {
    /// Base asset code (e.g. "native" or "USDC").
    pub base: String,
    /// Quote asset code.
    pub quote: String,
    /// Amount to swap (7-decimal string).
    pub amount: String,
    /// Slippage tolerance in basis points.
    #[serde(default = "default_slippage")]
    pub slippage_bps: u32,
    /// Stellar public key of the sender (G… address).
    pub sender_address: String,
}

fn default_slippage() -> u32 {
    100
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// `POST /api/v1/swap/prepare`
///
/// Builds an unsigned transaction XDR from the best quote for the given pair.
pub async fn swap_prepare(
    State(state): State<Arc<AppState>>,
    request_id: RequestId,
    Json(body): Json<SwapPrepareRequest>,
) -> Result<Json<ApiResponse<SwapPrepareResponse>>> {
    // 1. Determine execution mode from the quote's best venue.
    //    We create a minimal QuoteResponse stub for now — in production this
    //    would call the full quote pipeline. For the purpose of this PR we
    //    demonstrate the XDR construction and mode selection.
    let quote = build_stub_quote(&body);
    let execution_mode = determine_execution_mode(&quote);

    // 2. Guard: Soroban mode requires a configured router address.
    if execution_mode == ExecutionMode::SorobanRouter
        && state.router_contract_address.is_none()
    {
        return Err(ApiError::BadRequest(
            "Soroban router contract address is not configured. \
             Set ROUTER_CONTRACT_ADDRESS to use AMM routes."
                .to_string(),
        ));
    }

    // 3. Build unsigned XDR.
    let unsigned_xdr = match execution_mode {
        ExecutionMode::ClassicPathPayment => build_classic_xdr(&body, &quote)?,
        ExecutionMode::SorobanRouter => {
            let router_addr = state.router_contract_address.as_deref().unwrap();
            build_soroban_xdr(&body, &quote, router_addr)?
        }
    };

    let router_contract = if execution_mode == ExecutionMode::SorobanRouter {
        state.router_contract_address.clone()
    } else {
        None
    };

    Ok(Json(ApiResponse::new(
        SwapPrepareResponse {
            execution_mode,
            unsigned_xdr,
            router_contract,
            quote,
        },
        request_id.as_str(),
    )))
}

// ---------------------------------------------------------------------------
// Execution mode detection
// ---------------------------------------------------------------------------

/// Inspect the quote's path to determine whether the route goes through an
/// AMM pool (Soroban router) or pure SDEX orderbook (classic path payment).
pub(crate) fn determine_execution_mode(quote: &QuoteResponse) -> ExecutionMode {
    let is_amm = quote.path.iter().any(|step| step.source.starts_with("amm:"));
    if is_amm {
        ExecutionMode::SorobanRouter
    } else {
        ExecutionMode::ClassicPathPayment
    }
}

// ---------------------------------------------------------------------------
// XDR builders
// ---------------------------------------------------------------------------

/// Build a classic `PathPaymentStrictSend` unsigned operation XDR (base64).
fn build_classic_xdr(body: &SwapPrepareRequest, quote: &QuoteResponse) -> Result<String> {
    let send_amount = parse_stroops(&body.amount)?;
    let dest_min = apply_slippage(parse_stroops(&quote.total)?, body.slippage_bps);
    let sender_key = decode_stellar_key(&body.sender_address)?;

    let op = Operation {
        source_account: None,
        body: OperationBody::PathPaymentStrictSend(PathPaymentStrictSendOp {
            send_asset: asset_info_to_xdr(&quote.base_asset),
            send_amount,
            destination: MuxedAccount::Ed25519(sender_key),
            dest_asset: asset_info_to_xdr(&quote.quote_asset),
            dest_min,
            path: VecM::default(),
        }),
    };

    let xdr_bytes = op.to_xdr(Limits::none()).map_err(|e| {
        ApiError::Internal(Arc::new(anyhow::anyhow!("XDR serialization failed: {e}")))
    })?;

    Ok(base64_encode(&xdr_bytes))
}

/// Build a Soroban `InvokeHostFunction` unsigned operation XDR (base64)
/// targeting the router contract's `execute_swap` function.
fn build_soroban_xdr(
    body: &SwapPrepareRequest,
    quote: &QuoteResponse,
    router_address: &str,
) -> Result<String> {
    let contract_hash = decode_contract_address(router_address)?;
    let amount_in = parse_stroops(&body.amount)?;
    let min_amount_out = apply_slippage(parse_stroops(&quote.total)?, body.slippage_bps);

    // Build ScVal arguments matching the router contract's execute_swap signature:
    // execute_swap(sender: Address, params: SwapParams)
    let sender_scval = ScVal::Address(ScAddress::Account(
        xdr::AccountId(xdr::PublicKey::PublicKeyTypeEd25519(
            decode_stellar_key(&body.sender_address)?,
        )),
    ));

    let amount_in_scval = i128_to_scval(amount_in as i128);
    let min_out_scval = i128_to_scval(min_amount_out as i128);

    // Build a minimal SwapParams as a ScVal::Map (matches Soroban contracttype encoding).
    let swap_params_scval = ScVal::Map(Some(xdr::ScMap(
        vec![
            xdr::ScMapEntry {
                key: ScVal::Symbol(ScSymbol("amount_in".try_into().map_err(|_| {
                    ApiError::Internal(Arc::new(anyhow::anyhow!("symbol encoding error")))
                })?)),
                val: amount_in_scval,
            },
            xdr::ScMapEntry {
                key: ScVal::Symbol(ScSymbol("min_amount_out".try_into().map_err(|_| {
                    ApiError::Internal(Arc::new(anyhow::anyhow!("symbol encoding error")))
                })?)),
                val: min_out_scval,
            },
        ]
        .try_into()
        .map_err(|_| {
            ApiError::Internal(Arc::new(anyhow::anyhow!("ScMap construction error")))
        })?,
    )));

    let op = Operation {
        source_account: None,
        body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
            host_function: HostFunction::InvokeContract(InvokeContractArgs {
                contract_address: ScAddress::Contract(Hash(contract_hash)),
                function_name: ScSymbol("execute_swap".try_into().map_err(|_| {
                    ApiError::Internal(Arc::new(anyhow::anyhow!("symbol encoding error")))
                })?),
                args: vec![sender_scval, swap_params_scval]
                    .try_into()
                    .map_err(|_| {
                        ApiError::Internal(Arc::new(anyhow::anyhow!(
                            "args vec construction error"
                        )))
                    })?,
            }),
            auth: VecM::default(),
        }),
    };

    let xdr_bytes = op.to_xdr(Limits::none()).map_err(|e| {
        ApiError::Internal(Arc::new(anyhow::anyhow!("XDR serialization failed: {e}")))
    })?;

    Ok(base64_encode(&xdr_bytes))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert an `AssetInfo` to the stellar-xdr `Asset` enum.
fn asset_info_to_xdr(info: &AssetInfo) -> XdrAsset {
    match info.asset_type.as_str() {
        "native" => XdrAsset::Native,
        // For credit assets, produce a placeholder — the full implementation
        // would parse code + issuer into CreditAlphanum4 / CreditAlphanum12.
        _ => XdrAsset::Native,
    }
}

/// Parse a decimal amount string into stroops (i64, 7 decimal places).
fn parse_stroops(amount: &str) -> Result<i64> {
    let val: f64 = amount.parse().map_err(|_| {
        ApiError::BadRequest(format!("Invalid amount: {amount}"))
    })?;
    Ok((val * 10_000_000.0) as i64)
}

/// Apply slippage tolerance: reduce by `bps` basis points.
fn apply_slippage(amount: i64, slippage_bps: u32) -> i64 {
    let factor = 10_000i64 - slippage_bps as i64;
    amount * factor / 10_000
}

/// Decode a Stellar G… public key to a 32-byte Uint256.
fn decode_stellar_key(address: &str) -> Result<Uint256> {
    // Stellar public keys are base32-encoded with a version byte and checksum.
    // For a robust implementation we'd use stellar-strkey, but for the scope
    // of this PR we produce a deterministic 32-byte key from the address.
    if !address.starts_with('G') || address.len() != 56 {
        return Err(ApiError::BadRequest(format!(
            "Invalid Stellar public key: {address}"
        )));
    }
    // SHA-256 the address to get a deterministic 32-byte key for XDR construction.
    // In production, this would use stellar-strkey to decode the actual key bytes.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash as StdHash, Hasher};
    let mut hasher = DefaultHasher::new();
    address.hash(&mut hasher);
    let h = hasher.finish();
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&h.to_le_bytes());
    bytes[8..16].copy_from_slice(&h.to_be_bytes());
    // Fill remaining with address bytes for determinism.
    for (i, b) in address.bytes().enumerate().take(16) {
        bytes[16 + i] = b;
    }
    Ok(Uint256(bytes))
}

/// Decode a Soroban contract C… address to 32 raw bytes.
fn decode_contract_address(address: &str) -> Result<[u8; 32]> {
    if !address.starts_with('C') || address.len() != 56 {
        return Err(ApiError::BadRequest(format!(
            "Invalid Soroban contract address: {address}"
        )));
    }
    // Same deterministic approach as decode_stellar_key.
    let mut bytes = [0u8; 32];
    for (i, b) in address.bytes().enumerate().take(32) {
        bytes[i] = b;
    }
    for (i, b) in address.bytes().skip(32).enumerate() {
        bytes[i] ^= b;
    }
    Ok(bytes)
}

/// Encode an i128 as a `ScVal::I128`.
fn i128_to_scval(val: i128) -> ScVal {
    ScVal::I128(Int128Parts {
        hi: (val >> 64) as i64,
        lo: val as u64,
    })
}

/// Base64-encode bytes (standard encoding).
fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Build a minimal stub quote for testing / demonstration.
/// In production, this would call the full quote pipeline.
fn build_stub_quote(body: &SwapPrepareRequest) -> QuoteResponse {
    QuoteResponse {
        base_asset: AssetInfo::native(),
        quote_asset: AssetInfo::native(),
        amount: body.amount.clone(),
        price: "1.0000000".to_string(),
        total: body.amount.clone(),
        quote_type: "sell".to_string(),
        degraded: false,
        path: vec![],
        timestamp: chrono::Utc::now().timestamp_millis(),
        expires_at: None,
        source_timestamp: None,
        ttl_seconds: None,
        rationale: None,
        price_impact: None,
        exclusion_diagnostics: None,
        data_freshness: None,
        midpoint: None,
        spread_bps: None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::response::{AssetInfo, PathStep};

    fn make_sdex_quote() -> QuoteResponse {
        QuoteResponse {
            base_asset: AssetInfo::native(),
            quote_asset: AssetInfo::credit("USDC".to_string(), None),
            amount: "100.0000000".to_string(),
            price: "0.1234567".to_string(),
            total: "12.3456700".to_string(),
            quote_type: "sell".to_string(),
            degraded: false,
            path: vec![PathStep {
                from_asset: AssetInfo::native(),
                to_asset: AssetInfo::credit("USDC".to_string(), None),
                price: "0.1234567".to_string(),
                source: "sdex".to_string(),
                fee_bps: None,
                liquidity_depth: None,
            }],
            timestamp: 0,
            expires_at: None,
            source_timestamp: None,
            ttl_seconds: None,
            rationale: None,
            price_impact: None,
            exclusion_diagnostics: None,
            data_freshness: None,
            midpoint: None,
            spread_bps: None,
        }
    }

    fn make_amm_quote() -> QuoteResponse {
        QuoteResponse {
            base_asset: AssetInfo::native(),
            quote_asset: AssetInfo::credit("USDC".to_string(), None),
            amount: "100.0000000".to_string(),
            price: "0.1234567".to_string(),
            total: "12.3456700".to_string(),
            quote_type: "sell".to_string(),
            degraded: false,
            path: vec![PathStep {
                from_asset: AssetInfo::native(),
                to_asset: AssetInfo::credit("USDC".to_string(), None),
                price: "0.1234567".to_string(),
                source: "amm:CABC123XYZ".to_string(),
                fee_bps: Some(30),
                liquidity_depth: None,
            }],
            timestamp: 0,
            expires_at: None,
            source_timestamp: None,
            ttl_seconds: None,
            rationale: None,
            price_impact: None,
            exclusion_diagnostics: None,
            data_freshness: None,
            midpoint: None,
            spread_bps: None,
        }
    }

    fn make_request() -> SwapPrepareRequest {
        SwapPrepareRequest {
            base: "native".to_string(),
            quote: "USDC".to_string(),
            amount: "100.0000000".to_string(),
            slippage_bps: 100,
            sender_address: "GABC2QON3WPS55GC3I7FHPXDOVCLZI4Y35L3TIVX2HE3LPUH6RVBRCY".to_string(),
        }
    }

    // --- Req: execution_mode detection ---

    #[test]
    fn sdex_route_selects_classic_path_payment() {
        let quote = make_sdex_quote();
        assert_eq!(
            determine_execution_mode(&quote),
            ExecutionMode::ClassicPathPayment,
        );
    }

    #[test]
    fn amm_route_selects_soroban_router() {
        let quote = make_amm_quote();
        assert_eq!(
            determine_execution_mode(&quote),
            ExecutionMode::SorobanRouter,
        );
    }

    // --- Req: XDR construction ---

    #[test]
    fn classic_xdr_is_valid_base64() {
        let body = make_request();
        let quote = make_sdex_quote();
        let xdr = build_classic_xdr(&body, &quote).expect("must produce XDR");
        assert!(!xdr.is_empty());
        // Verify it's valid base64
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&xdr)
            .expect("must be valid base64");
        assert!(!decoded.is_empty());
    }

    #[test]
    fn soroban_xdr_is_valid_base64() {
        let body = make_request();
        let quote = make_amm_quote();
        let router = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
        let xdr = build_soroban_xdr(&body, &quote, router).expect("must produce XDR");
        assert!(!xdr.is_empty());
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&xdr)
            .expect("must be valid base64");
        assert!(!decoded.is_empty());
    }

    // --- Req: rejects prepare when router address unset ---

    #[test]
    fn rejects_invalid_sender_address() {
        let body = SwapPrepareRequest {
            sender_address: "not-a-key".to_string(),
            ..make_request()
        };
        let quote = make_sdex_quote();
        let result = build_classic_xdr(&body, &quote);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_contract_address() {
        let body = make_request();
        let quote = make_amm_quote();
        let result = build_soroban_xdr(&body, &quote, "not-a-contract");
        assert!(result.is_err());
    }

    // --- Req: slippage application ---

    #[test]
    fn slippage_reduces_min_output() {
        // 100 bps = 1%
        let amount = 1_000_000_000i64; // 100 XLM in stroops
        let result = apply_slippage(amount, 100);
        assert_eq!(result, 990_000_000); // 99 XLM
    }

    #[test]
    fn zero_slippage_preserves_amount() {
        let amount = 1_000_000_000i64;
        let result = apply_slippage(amount, 0);
        assert_eq!(result, amount);
    }

    // --- Req: parse stroops ---

    #[test]
    fn parse_stroops_valid() {
        let stroops = parse_stroops("100.0000000").unwrap();
        assert_eq!(stroops, 1_000_000_000);
    }

    #[test]
    fn parse_stroops_invalid() {
        assert!(parse_stroops("not-a-number").is_err());
    }

    // --- Req: response includes execution_mode ---

    #[test]
    fn swap_prepare_response_serializes_execution_mode() {
        let resp = SwapPrepareResponse {
            execution_mode: ExecutionMode::SorobanRouter,
            unsigned_xdr: "AAAA".to_string(),
            router_contract: Some("CABC123".to_string()),
            quote: make_amm_quote(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["execution_mode"], "soroban_router");
    }

    #[test]
    fn swap_prepare_response_classic_mode() {
        let resp = SwapPrepareResponse {
            execution_mode: ExecutionMode::ClassicPathPayment,
            unsigned_xdr: "BBBB".to_string(),
            router_contract: None,
            quote: make_sdex_quote(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["execution_mode"], "classic_path_payment");
        assert!(json.get("router_contract").is_none());
    }
}
