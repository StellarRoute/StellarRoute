//! Strict Soroban `simulateTransaction` + unsigned envelope assembly for CCTP builders.
//!
//! Production builders require successful simulation with exactly one result, bounded
//! `transactionData`/`minResourceFee`, and reject restore-preamble requirements.

use serde::Deserialize;
use serde_json::json;
use stellar_xdr::curr::{
    LedgerBounds, Limits, Operation, OperationBody, ReadXdr, SorobanAuthorizationEntry,
    SorobanTransactionData, TimeBounds, TimePoint, Transaction, TransactionEnvelope,
    TransactionExt, TransactionV1Envelope, VecM, WriteXdr,
};

use crate::cctp::builders::stellar::encoder::{
    build_unsigned_invoke_tx, InvokeTxParams, MAX_AUTH_ENTRIES,
};
use crate::cctp::builders::BuilderError;
use crate::cctp::config::CctpConfig;
use crate::cctp::stellar_rpc::{check_rpc_response_len, StellarRpcClient};

/// Margin ledgers before persisted approval expiration when re-approval is required.
pub const APPROVAL_LEDGER_SAFETY_MARGIN: u32 = 32;

/// Margin added to latest ledger for approval expiration and tx ledger bounds.
pub const LEDGER_EXPIRY_MARGIN: u32 = 1_000;

#[derive(Debug, Clone)]
pub struct StrictSimulateResult {
    pub transaction_data_xdr: String,
    pub min_resource_fee: u64,
    pub auth_entries: Vec<SorobanAuthorizationEntry>,
}

pub fn ledger_bounds_for_expiry(latest_ledger: u32, quote_expires_at: i64) -> LedgerBounds {
    let now = chrono::Utc::now().timestamp();
    let secs_remaining = quote_expires_at.saturating_sub(now).max(0) as u32;
    // Testnet ~5s/ledger; add margin so bounds survive quote TTL.
    let ledger_margin = secs_remaining / 5 + LEDGER_EXPIRY_MARGIN;
    LedgerBounds {
        min_ledger: latest_ledger,
        max_ledger: latest_ledger.saturating_add(ledger_margin),
    }
}

pub fn approval_expiration_ledger(latest_ledger: u32, quote_expires_at: i64) -> u32 {
    ledger_bounds_for_expiry(latest_ledger, quote_expires_at).max_ledger
}

pub fn time_bounds_for_expiry(quote_expires_at: i64) -> TimeBounds {
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    let max = quote_expires_at.max(now as i64) as u64;
    TimeBounds {
        min_time: TimePoint(0),
        max_time: TimePoint(max),
    }
}

pub fn compute_total_fee(base_fee: u32, min_resource_fee: u64) -> Result<u32, BuilderError> {
    let total = base_fee as u64 + min_resource_fee;
    u32::try_from(total).map_err(|_| BuilderError::SimulationFailed("fee overflow".into()))
}

impl StellarRpcClient {
    pub async fn simulate_transaction_strict(
        &self,
        transaction_xdr: &str,
    ) -> Result<StrictSimulateResult, BuilderError> {
        self.ensure_url(&self.rpc_url)
            .map_err(|e| BuilderError::SimulationFailed(e.to_string()))?;
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "simulateTransaction",
            "params": {
                "transaction": transaction_xdr,
                "resourceConfig": { "instructionLeeway": 1_000_000 }
            }
        });
        let body_str = body.to_string();
        if body_str.len() > crate::cctp::stellar_rpc::MAX_JSON_BODY_BYTES {
            return Err(BuilderError::SimulationFailed("request too large".into()));
        }
        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| BuilderError::SimulationFailed(e.to_string()))?;
        let text = resp
            .text()
            .await
            .map_err(|e| BuilderError::SimulationFailed(e.to_string()))?;
        check_rpc_response_len(text.len())
            .map_err(|e| BuilderError::SimulationFailed(e.to_string()))?;

        #[derive(Deserialize)]
        struct RpcResponse {
            result: Option<SimPayload>,
            error: Option<RpcError>,
        }
        #[derive(Deserialize)]
        struct RpcError {
            message: String,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SimPayload {
            transaction_data: Option<String>,
            min_resource_fee: Option<String>,
            results: Option<Vec<SimResultItem>>,
            restore_preamble: Option<serde_json::Value>,
            error: Option<String>,
        }
        #[derive(Deserialize)]
        struct SimResultItem {
            auth: Option<Vec<String>>,
            error: Option<String>,
        }

        let payload: RpcResponse = serde_json::from_str(&text)
            .map_err(|e| BuilderError::SimulationFailed(e.to_string()))?;
        if let Some(err) = payload.error {
            return Err(BuilderError::SimulationFailed(err.message));
        }
        let result = payload
            .result
            .ok_or_else(|| BuilderError::SimulationFailed("missing result".into()))?;
        if result.restore_preamble.is_some() {
            return Err(BuilderError::SimulationFailed(
                "restore preamble required".into(),
            ));
        }
        if let Some(err) = result.error {
            return Err(BuilderError::SimulationFailed(err));
        }
        let transaction_data = result
            .transaction_data
            .filter(|s| !s.is_empty())
            .ok_or_else(|| BuilderError::SimulationFailed("missing transactionData".into()))?;
        if transaction_data.len() > 512 * 1024 {
            return Err(BuilderError::SimulationFailed(
                "transactionData too large".into(),
            ));
        }
        let min_resource_fee = result
            .min_resource_fee
            .as_deref()
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|_| BuilderError::SimulationFailed("invalid minResourceFee".into()))?;
        let results = result
            .results
            .ok_or_else(|| BuilderError::SimulationFailed("missing results".into()))?;
        if results.is_empty() {
            return Err(BuilderError::SimulationFailed("empty results".into()));
        }
        if results.len() != 1 {
            return Err(BuilderError::SimulationFailed(format!(
                "expected 1 result, got {}",
                results.len()
            )));
        }
        if let Some(err) = results[0].error.as_deref() {
            if !err.is_empty() {
                return Err(BuilderError::SimulationFailed(err.to_string()));
            }
        }
        let mut auth_entries = Vec::new();
        if let Some(auth_xdrs) = results[0].auth.as_ref() {
            if auth_xdrs.len() > MAX_AUTH_ENTRIES {
                return Err(BuilderError::SimulationFailed(
                    "too many auth entries".into(),
                ));
            }
            for xdr in auth_xdrs {
                if xdr.len() > 256 * 1024 {
                    return Err(BuilderError::SimulationFailed("auth xdr too large".into()));
                }
                let entry = SorobanAuthorizationEntry::from_xdr_base64(xdr, Limits::none())
                    .map_err(|e| BuilderError::SimulationFailed(e.to_string()))?;
                auth_entries.push(entry);
            }
        }
        Ok(StrictSimulateResult {
            transaction_data_xdr: transaction_data,
            min_resource_fee,
            auth_entries,
        })
    }
}

pub fn assemble_simulated_envelope(
    mut tx: Transaction,
    sim: &StrictSimulateResult,
    base_fee: u32,
) -> Result<String, BuilderError> {
    let soroban_data =
        SorobanTransactionData::from_xdr_base64(&sim.transaction_data_xdr, Limits::none())
            .map_err(|e| BuilderError::SimulationFailed(e.to_string()))?;
    tx.fee = compute_total_fee(base_fee, sim.min_resource_fee)?;
    tx.ext = TransactionExt::V1(soroban_data);

    let first = tx
        .operations
        .first()
        .cloned()
        .ok_or_else(|| BuilderError::Encoding("missing operation".into()))?;
    let OperationBody::InvokeHostFunction(mut invoke) = first.body else {
        return Err(BuilderError::Encoding("expected invoke".into()));
    };
    invoke.auth = sim
        .auth_entries
        .clone()
        .try_into()
        .map_err(|_| BuilderError::SimulationFailed("auth vec".into()))?;
    tx.operations = vec![Operation {
        source_account: first.source_account,
        body: OperationBody::InvokeHostFunction(invoke),
    }]
    .try_into()
    .map_err(|_| BuilderError::Encoding("operation vec".into()))?;

    let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        signatures: VecM::default(),
    });
    envelope
        .to_xdr_base64(Limits::none())
        .map_err(|e| BuilderError::Encoding(e.to_string()))
}

pub async fn simulate_and_assemble_invoke(
    rpc: &StellarRpcClient,
    params: InvokeTxParams,
) -> Result<String, BuilderError> {
    let tx = build_unsigned_invoke_tx(&params)?;
    let template_xdr = TransactionEnvelope::Tx(TransactionV1Envelope {
        tx: tx.clone(),
        signatures: VecM::default(),
    })
    .to_xdr_base64(Limits::none())
    .map_err(|e| BuilderError::Encoding(e.to_string()))?;
    let sim = rpc.simulate_transaction_strict(&template_xdr).await?;
    assemble_simulated_envelope(tx, &sim, params.base_fee)
}

/// Live readiness: strict simulate→assemble on USDC `decimals` (read-only).
pub async fn probe_strict_simulation_assembly(rpc: &StellarRpcClient, config: &CctpConfig) -> bool {
    probe_strict_simulation_assembly_with_evidence(rpc, config, None)
        .await
        .is_ok()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StrictSimulationEvidence {
    pub source_public_key: String,
    pub latest_ledger: u32,
    pub sequence: i64,
    pub contract: String,
    pub method: String,
    pub simulation_success: bool,
    pub auth_entry_count: usize,
    pub result_count: usize,
    pub min_resource_fee: u64,
    pub assembled_envelope_sha256: String,
    pub assembled_envelope_size_bytes: usize,
}

pub async fn probe_strict_simulation_assembly_with_evidence(
    rpc: &StellarRpcClient,
    config: &CctpConfig,
    source_override: Option<&str>,
) -> Result<StrictSimulationEvidence, String> {
    use crate::cctp::fixtures::stellar_live_xdr::mint_operation_source;
    use crate::cctp::stellar_sequence::RpcAccountSequenceSource;
    use crate::swap::tx::AccountSequenceSource;
    use sha2::{Digest, Sha256};
    use std::sync::Arc;

    if config.stellar_rpc_url.trim().is_empty() {
        return Err("empty rpc url".into());
    }
    let source = source_override
        .map(str::to_string)
        .unwrap_or_else(mint_operation_source);
    let latest = rpc
        .latest_ledger()
        .await
        .map_err(|e| format!("latest_ledger: {e}"))?;
    let sequences = RpcAccountSequenceSource::new(config, Arc::new(rpc.clone()));
    let current = sequences
        .current_sequence(&source)
        .await
        .map_err(|e| format!("sequence: {e}"))?;
    let sequence = current.saturating_add(1);
    let quote_exp = chrono::Utc::now().timestamp() + 600;
    let contract = config.contracts.stellar_usdc.clone();
    let params = InvokeTxParams {
        source: source.clone(),
        contract: contract.clone(),
        function: "decimals".to_string(),
        args: vec![],
        sequence,
        base_fee: crate::swap::tx::DEFAULT_BASE_FEE,
        time_bounds: time_bounds_for_expiry(quote_exp),
        ledger_bounds: ledger_bounds_for_expiry(latest, quote_exp),
    };
    let sim = rpc
        .simulate_transaction_strict(
            &TransactionEnvelope::Tx(TransactionV1Envelope {
                tx: build_unsigned_invoke_tx(&params).map_err(|e| format!("build tx: {e}"))?,
                signatures: VecM::default(),
            })
            .to_xdr_base64(Limits::none())
            .map_err(|e| format!("template xdr: {e}"))?,
        )
        .await
        .map_err(|e| format!("simulate: {e}"))?;
    let assembled = simulate_and_assemble_invoke(rpc, params)
        .await
        .map_err(|e| format!("assemble: {e}"))?;
    TransactionEnvelope::from_xdr_base64(&assembled, Limits::none())
        .map_err(|e| format!("decode assembled envelope: {e}"))?;
    let envelope_hash = hex::encode(Sha256::digest(assembled.as_bytes()));
    Ok(StrictSimulationEvidence {
        source_public_key: source,
        latest_ledger: latest,
        sequence,
        contract,
        method: "decimals".into(),
        simulation_success: true,
        auth_entry_count: sim.auth_entries.len(),
        result_count: 1,
        min_resource_fee: sim.min_resource_fee,
        assembled_envelope_sha256: envelope_hash,
        assembled_envelope_size_bytes: assembled.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::builders::stellar::encoder::{
        account_address, approve_args, contract_address,
    };
    use crate::cctp::config::CctpConfig;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn ledger_bounds_respect_quote_expiry() {
        let future = chrono::Utc::now().timestamp() + 600;
        let bounds = ledger_bounds_for_expiry(100, future);
        assert_eq!(bounds.min_ledger, 100);
        assert!(bounds.max_ledger > 100);
    }

    #[test]
    fn fee_overflow_rejected() {
        assert!(compute_total_fee(u32::MAX, 1).is_err());
    }

    #[tokio::test]
    async fn rejects_restore_preamble() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = server.uri();
        let rpc = StellarRpcClient::new(&cfg).unwrap();
        Mock::given(method("POST"))
            .and(body_string_contains("simulateTransaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0", "id": 1,
                "result": {
                    "transactionData": "AAAAAgAAAADoW5bEWSrMfFQYxjx7qQIRhAvYGC53Ncv7o6mif8E88gAAAGQSDz3wAAAAZAAAAAEAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAABAAAAAAAAAAAAAAAFAAAAAA==",
                    "minResourceFee": "100",
                    "results": [{ "xdr": "AAAAAA==" }],
                    "restorePreamble": { "transactionData": "AAAA" }
                }
            })))
            .mount(&server)
            .await;
        let err = rpc.simulate_transaction_strict("AAAA").await.unwrap_err();
        assert!(matches!(err, BuilderError::SimulationFailed(ref m) if m.contains("restore")));
    }

    #[tokio::test]
    async fn rejects_empty_results() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = server.uri();
        let rpc = StellarRpcClient::new(&cfg).unwrap();
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0", "id": 1,
                "result": {
                    "transactionData": "AAAAAgAAAADoW5bEWSrMfFQYxjx7qQIRhAvYGC53Ncv7o6mif8E88gAAAGQSDz3wAAAAZAAAAAEAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAABAAAAAAAAAAAAAAAFAAAAAA==",
                    "minResourceFee": "0",
                    "results": []
                }
            })))
            .mount(&server)
            .await;
        let err = rpc.simulate_transaction_strict("AAAA").await.unwrap_err();
        assert!(matches!(err, BuilderError::SimulationFailed(ref m) if m.contains("empty")));
    }

    #[test]
    fn approve_args_include_expiration_ledger() {
        let args = approve_args(
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            "CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP",
            1_000_000,
            9_999,
        )
        .unwrap();
        assert_eq!(args.len(), 4);
        assert!(matches!(args[3], stellar_xdr::curr::ScVal::U32(9_999)));
    }
}
