//! Minimal Soroban JSON-RPC client for read-only contract simulation.
//!
//! Redirect policy: no redirects; URL must match configured host exactly.

use reqwest::redirect::Policy;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use stellar_xdr::curr::{ReadXdr, ScVal};

use crate::cctp::builders::stellar::encoder::encode_invoke_at_sequence;
use crate::cctp::config::{parse_service_url, CctpConfig};
use crate::cctp::verifiers::VerifierError;

pub const MAX_JSON_BODY_BYTES: usize = 256 * 1024;
/// Soroban simulate responses can include large XDR payloads; bounded separately from requests.
pub const MAX_JSON_RESPONSE_BYTES: usize = 1024 * 1024;
pub const SIMULATE_SOURCE: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

#[derive(Debug, Clone)]
pub struct StellarRpcClient {
    pub client: Client,
    pub rpc_url: String,
    pub allowed_host: String,
    pub network_passphrase: String,
}

impl StellarRpcClient {
    pub fn new(config: &CctpConfig) -> Result<Self, VerifierError> {
        if config.stellar_rpc_url.trim().is_empty() {
            return Err(VerifierError::NotReady);
        }
        let parsed = parse_service_url(&config.stellar_rpc_url)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        if !cfg!(test) && parsed.scheme != "https" {
            return Err(VerifierError::Failed("stellar rpc must be https".into()));
        }
        Ok(Self {
            client: Client::builder()
                .redirect(Policy::none())
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|e| VerifierError::Transient(e.to_string()))?,
            rpc_url: config.stellar_rpc_url.clone(),
            allowed_host: parsed.host,
            network_passphrase: config.stellar_network_passphrase.clone(),
        })
    }

    pub fn is_ready(&self) -> bool {
        !self.rpc_url.trim().is_empty()
    }

    pub(crate) fn ensure_url(&self, url: &str) -> Result<(), VerifierError> {
        let parsed = parse_service_url(url).map_err(|_| VerifierError::Failed("rpc url".into()))?;
        if parsed.host != self.allowed_host {
            return Err(VerifierError::Failed("rpc host mismatch".into()));
        }
        if !cfg!(test) && parsed.scheme != "https" {
            return Err(VerifierError::Failed("rpc scheme".into()));
        }
        Ok(())
    }

    async fn call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, VerifierError> {
        self.ensure_url(&self.rpc_url)?;
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let body_str = body.to_string();
        if body_str.len() > MAX_JSON_BODY_BYTES {
            return Err(VerifierError::Failed("rpc request too large".into()));
        }
        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| VerifierError::Transient(e.to_string()))?;
        let text = resp
            .text()
            .await
            .map_err(|e| VerifierError::Transient(e.to_string()))?;
        check_rpc_response_len(text.len())?;
        #[derive(Deserialize)]
        struct RpcResponse<T> {
            result: Option<T>,
            error: Option<RpcError>,
        }
        #[derive(Deserialize)]
        struct RpcError {
            message: String,
        }
        let payload: RpcResponse<T> =
            serde_json::from_str(&text).map_err(|e| VerifierError::Failed(e.to_string()))?;
        if let Some(err) = payload.error {
            return Err(VerifierError::Failed(err.message));
        }
        payload.result.ok_or(VerifierError::TxNotFound)
    }

    pub async fn latest_ledger(&self) -> Result<u32, VerifierError> {
        #[derive(Deserialize)]
        struct LatestLedger {
            sequence: u32,
        }
        let result: LatestLedger = self.call("getLatestLedger", json!({})).await?;
        Ok(result.sequence)
    }

    /// Current on-ledger account sequence (Horizon/RPC value; next tx uses +1).
    pub async fn get_account_sequence(&self, account_id: &str) -> Result<i64, VerifierError> {
        use stellar_xdr::curr::{
            AccountId, LedgerEntry, LedgerEntryData, LedgerKey, LedgerKeyAccount, Limits,
            PublicKey, ReadXdr, Uint256, WriteXdr,
        };
        let pk = stellar_strkey::ed25519::PublicKey::from_string(account_id.trim())
            .map_err(|_| VerifierError::Failed("invalid G-address".into()))?;
        let account = AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(pk.0)));
        let key = LedgerKey::Account(LedgerKeyAccount {
            account_id: account,
        });
        let key_xdr = key
            .to_xdr_base64(Limits::none())
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        #[derive(Deserialize)]
        struct EntryResult {
            entries: Vec<EntryItem>,
        }
        #[derive(Deserialize)]
        struct EntryItem {
            xdr: String,
        }
        let result: EntryResult = self
            .call("getLedgerEntries", json!({ "keys": [key_xdr] }))
            .await?;
        let item = result
            .entries
            .first()
            .ok_or_else(|| VerifierError::Failed("account not found".into()))?;
        if item.xdr.len() > 256 * 1024 {
            return Err(VerifierError::Failed("ledger entry too large".into()));
        }
        let entry = LedgerEntry::from_xdr_base64(&item.xdr, Limits::none())
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        let LedgerEntryData::Account(account_entry) = entry.data else {
            return Err(VerifierError::Failed("not an account entry".into()));
        };
        Ok(account_entry.seq_num.0)
    }

    pub async fn simulate_scval(
        &self,
        contract: &str,
        function: &str,
        args: Vec<stellar_xdr::curr::ScVal>,
    ) -> Result<ScVal, VerifierError> {
        let ledger = self.latest_ledger().await?;
        let xdr =
            encode_invoke_at_sequence(SIMULATE_SOURCE, contract, function, args, ledger as i64)
                .map_err(|e| VerifierError::Failed(e.to_string()))?;
        #[derive(Deserialize)]
        struct SimulateResult {
            results: Vec<SimItem>,
        }
        #[derive(Deserialize)]
        struct SimItem {
            xdr: String,
        }
        let result: SimulateResult = self
            .call(
                "simulateTransaction",
                json!({
                    "transaction": xdr,
                    "resourceConfig": { "instructionLeeway": 1_000_000 }
                }),
            )
            .await?;
        let item = result
            .results
            .first()
            .ok_or(VerifierError::Failed("no sim result".into()))?;
        let scval = ScVal::from_xdr_base64(&item.xdr, stellar_xdr::curr::Limits::none())
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        Ok(scval)
    }
}

pub fn scval_to_u32(val: &ScVal) -> Result<u32, VerifierError> {
    match val {
        ScVal::U32(v) => Ok(*v),
        ScVal::Void => Err(VerifierError::Failed("option none".into())),
        _ => Err(VerifierError::Failed("expected u32".into())),
    }
}

pub fn scval_to_option_u32(val: &ScVal) -> Result<Option<u32>, VerifierError> {
    match val {
        ScVal::U32(v) => Ok(Some(*v)),
        ScVal::Void => Ok(None),
        ScVal::I32(v) if *v >= 0 => Ok(Some(*v as u32)),
        _ => Err(VerifierError::Failed("expected option u32".into())),
    }
}

pub fn scval_to_bool(val: &ScVal) -> Result<bool, VerifierError> {
    match val {
        ScVal::Bool(v) => Ok(*v),
        _ => Err(VerifierError::Failed("expected bool".into())),
    }
}

pub fn scval_to_bytes20(val: &ScVal) -> Result<[u8; 20], VerifierError> {
    use stellar_xdr::curr::ScBytes;
    match val {
        ScVal::Bytes(ScBytes(bytes)) if bytes.len() == 20 => {
            let mut out = [0u8; 20];
            out.copy_from_slice(bytes);
            Ok(out)
        }
        _ => Err(VerifierError::Failed("expected bytes20".into())),
    }
}

pub fn bytes20_scval(bytes: [u8; 20]) -> stellar_xdr::curr::ScVal {
    use stellar_xdr::curr::ScBytes;
    stellar_xdr::curr::ScVal::Bytes(ScBytes(
        bytes
            .to_vec()
            .try_into()
            .unwrap_or_else(|_| panic!("bytes20")),
    ))
}

pub fn u32_scval(value: u32) -> stellar_xdr::curr::ScVal {
    stellar_xdr::curr::ScVal::U32(value)
}

pub(crate) fn check_rpc_response_len(len: usize) -> Result<(), VerifierError> {
    if len > MAX_JSON_RESPONSE_BYTES {
        return Err(VerifierError::Failed("rpc response too large".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::WriteXdr;

    #[test]
    fn decodes_option_u32_some_and_none() {
        assert_eq!(scval_to_option_u32(&ScVal::U32(7)).unwrap(), Some(7));
        assert_eq!(scval_to_option_u32(&ScVal::Void).unwrap(), None);
    }

    #[test]
    fn decodes_simulated_rpc_envelope_base64() {
        let val = ScVal::Bool(true);
        let xdr = val
            .to_xdr_base64(stellar_xdr::curr::Limits::none())
            .unwrap();
        let decoded = ScVal::from_xdr_base64(&xdr, stellar_xdr::curr::Limits::none()).unwrap();
        assert!(scval_to_bool(&decoded).unwrap());
    }

    #[test]
    fn decodes_bytes20_from_scval() {
        let bytes = [0xAB; 20];
        let val = bytes20_scval(bytes);
        assert_eq!(scval_to_bytes20(&val).unwrap(), bytes);
    }

    #[test]
    fn rejects_redirecting_client_policy() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = "https://soroban-testnet.stellar.org".into();
        let client = StellarRpcClient::new(&cfg).unwrap();
        assert_eq!(client.allowed_host, "soroban-testnet.stellar.org");
    }

    #[test]
    fn response_cap_accepts_at_limit() {
        assert!(check_rpc_response_len(MAX_JSON_RESPONSE_BYTES).is_ok());
    }

    #[test]
    fn response_cap_rejects_over_limit() {
        let err = check_rpc_response_len(MAX_JSON_RESPONSE_BYTES + 1).unwrap_err();
        assert!(matches!(err, VerifierError::Failed(msg) if msg == "rpc response too large"));
    }

    #[test]
    fn request_cap_unchanged_at_256_kib() {
        assert_eq!(MAX_JSON_BODY_BYTES, 256 * 1024);
        assert_eq!(MAX_JSON_RESPONSE_BYTES, 1024 * 1024);
    }
}
