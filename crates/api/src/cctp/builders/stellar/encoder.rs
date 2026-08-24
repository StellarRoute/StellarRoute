//! Shared Soroban invoke XDR encoding (no RPC, no simulation).

use chrono::Utc;
use stellar_xdr::curr::{
    AccountId, ContractId, Duration, Hash, HostFunction, InvokeContractArgs, InvokeHostFunctionOp,
    LedgerBounds, Limits, Memo, MuxedAccount, Operation, OperationBody, Preconditions,
    PreconditionsV2, PublicKey, ScAddress, ScBytes, ScSymbol, ScVal, SequenceNumber, TimeBounds,
    TimePoint, Transaction, TransactionEnvelope, TransactionExt, TransactionV1Envelope, Uint256,
    VecM, WriteXdr,
};

use crate::cctp::builders::BuilderError;
use crate::cctp::expectations::ANY_DESTINATION_CALLER;
use crate::swap::tx::{DEFAULT_BASE_FEE, DEFAULT_TIMEOUT_SECS};

pub const MAX_SIM_RESULTS: usize = 4;
pub const MAX_AUTH_ENTRIES: usize = 16;

pub fn contract_address(strkey: &str) -> Result<ScAddress, BuilderError> {
    let contract = stellar_strkey::Contract::from_string(strkey.trim())
        .map_err(|_| BuilderError::Validation(format!("invalid contract: {strkey}")))?;
    Ok(ScAddress::Contract(ContractId(Hash(contract.0))))
}

pub fn account_address(g: &str) -> Result<ScAddress, BuilderError> {
    let pk = stellar_strkey::ed25519::PublicKey::from_string(g.trim())
        .map_err(|_| BuilderError::Validation(format!("invalid G-address: {g}")))?;
    let account_id = AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(pk.0)));
    Ok(ScAddress::Account(account_id))
}

fn sc_symbol(name: &str) -> Result<ScSymbol, BuilderError> {
    ScSymbol::try_from(name.to_string())
        .map_err(|_| BuilderError::Encoding(format!("invalid symbol: {name}")))
}

fn bytes32_scval(bytes: [u8; 32]) -> ScVal {
    ScVal::Bytes(ScBytes(
        bytes
            .to_vec()
            .try_into()
            .unwrap_or_else(|_| panic!("bytes32 length")),
    ))
}

fn i128_scval(v: i128) -> ScVal {
    ScVal::I128(stellar_xdr::curr::Int128Parts {
        hi: (v >> 64) as i64,
        lo: v as u64,
    })
}

fn u32_scval(v: u32) -> ScVal {
    ScVal::U32(v)
}

/// SEP-41 `approve(from, spender, amount, live_until_ledger)` invoke args.
pub fn approve_args(
    owner: &str,
    spender_contract: &str,
    amount: i128,
    expiration_ledger: u32,
) -> Result<Vec<ScVal>, BuilderError> {
    Ok(vec![
        ScVal::Address(account_address(owner)?),
        ScVal::Address(contract_address(spender_contract)?),
        i128_scval(amount),
        u32_scval(expiration_ledger),
    ])
}

pub fn deposit_for_burn_args(
    caller: &str,
    amount_stellar: i128,
    destination_domain: u32,
    mint_recipient: [u8; 32],
    burn_token: &str,
    max_fee_stellar: i128,
    min_finality: u32,
) -> Result<Vec<ScVal>, BuilderError> {
    Ok(vec![
        ScVal::Address(account_address(caller)?),
        i128_scval(amount_stellar),
        u32_scval(destination_domain),
        bytes32_scval(mint_recipient),
        ScVal::Address(contract_address(burn_token)?),
        bytes32_scval(ANY_DESTINATION_CALLER),
        i128_scval(max_fee_stellar),
        u32_scval(min_finality),
    ])
}

pub fn mint_and_forward_args(
    message: &[u8],
    attestation: &[u8],
) -> Result<Vec<ScVal>, BuilderError> {
    Ok(vec![
        ScVal::Bytes(ScBytes(
            message
                .to_vec()
                .try_into()
                .map_err(|_| BuilderError::Encoding("message too large".into()))?,
        )),
        ScVal::Bytes(ScBytes(attestation.to_vec().try_into().map_err(|_| {
            BuilderError::Encoding("attestation too large".into())
        })?)),
    ])
}

#[derive(Debug, Clone)]
pub struct InvokeTxParams {
    pub source: String,
    pub contract: String,
    pub function: String,
    pub args: Vec<ScVal>,
    /// Horizon/RPC account sequence for the next transaction.
    pub sequence: i64,
    pub base_fee: u32,
    pub time_bounds: TimeBounds,
    pub ledger_bounds: LedgerBounds,
}

pub fn build_unsigned_invoke_tx(params: &InvokeTxParams) -> Result<Transaction, BuilderError> {
    let invoke = InvokeHostFunctionOp {
        host_function: HostFunction::InvokeContract(InvokeContractArgs {
            contract_address: contract_address(&params.contract)?,
            function_name: sc_symbol(&params.function)?,
            args: params
                .args
                .clone()
                .try_into()
                .map_err(|_| BuilderError::Encoding("too many contract args".into()))?,
        }),
        auth: VecM::default(),
    };

    Ok(Transaction {
        source_account: MuxedAccount::Ed25519(Uint256(
            stellar_strkey::ed25519::PublicKey::from_string(&params.source)
                .map_err(|_| BuilderError::Validation("invalid source".into()))?
                .0,
        )),
        fee: params.base_fee,
        seq_num: SequenceNumber(params.sequence),
        cond: Preconditions::V2(PreconditionsV2 {
            time_bounds: Some(params.time_bounds.clone()),
            ledger_bounds: Some(params.ledger_bounds.clone()),
            min_seq_num: None,
            min_seq_age: Duration(0),
            min_seq_ledger_gap: 0,
            extra_signers: VecM::default(),
        }),
        memo: Memo::None,
        operations: vec![Operation {
            source_account: None,
            body: OperationBody::InvokeHostFunction(invoke),
        }]
        .try_into()
        .map_err(|_| BuilderError::Encoding("operation vec".into()))?,
        ext: TransactionExt::V0,
    })
}

/// Encode unsigned invoke envelope at explicit next-transaction sequence number.
pub fn encode_invoke_at_sequence(
    source: &str,
    contract: &str,
    function: &str,
    args: Vec<ScVal>,
    account_sequence: i64,
) -> Result<String, BuilderError> {
    let now = Utc::now().timestamp() as u64;
    let tx = build_unsigned_invoke_tx(&InvokeTxParams {
        source: source.to_string(),
        contract: contract.to_string(),
        function: function.to_string(),
        args,
        sequence: account_sequence,
        base_fee: DEFAULT_BASE_FEE,
        time_bounds: TimeBounds {
            min_time: TimePoint(0),
            max_time: TimePoint(now + DEFAULT_TIMEOUT_SECS),
        },
        ledger_bounds: LedgerBounds {
            min_ledger: 0,
            max_ledger: u32::MAX,
        },
    })?;
    let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        signatures: VecM::default(),
    });
    envelope
        .to_xdr_base64(Limits::none())
        .map_err(|e| BuilderError::Encoding(e.to_string()))
}

pub fn envelope_sequence(xdr: &str) -> Result<i64, BuilderError> {
    use stellar_xdr::curr::ReadXdr;
    let env = TransactionEnvelope::from_xdr_base64(xdr, Limits::none())
        .map_err(|e| BuilderError::Encoding(e.to_string()))?;
    let TransactionEnvelope::Tx(v1) = env else {
        return Err(BuilderError::Encoding("expected v1 envelope".into()));
    };
    Ok(v1.tx.seq_num.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::config::{FINALITY_FAST, FINALITY_STANDARD};

    #[test]
    fn approve_args_sep41_tuple() {
        let args = approve_args(
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            "CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP",
            1,
            42,
        )
        .unwrap();
        assert_eq!(args.len(), 4);
    }

    #[test]
    fn deposit_for_burn_has_eight_args() {
        let args = deposit_for_burn_args(
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            1_000_000,
            0,
            [1u8; 32],
            "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
            1,
            FINALITY_STANDARD,
        )
        .unwrap();
        assert_eq!(args.len(), 8);
        assert_eq!(args[7], u32_scval(FINALITY_STANDARD));
    }

    #[test]
    fn deposit_for_burn_encodes_fast_finality() {
        let args = deposit_for_burn_args(
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            1_000_000,
            0,
            [1u8; 32],
            "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
            0,
            FINALITY_FAST,
        )
        .unwrap();
        assert_eq!(args[7], u32_scval(FINALITY_FAST));
    }

    #[test]
    fn distinct_sequences_for_approval_then_burn() {
        let source = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
        let contract = "CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP";
        let approve = encode_invoke_at_sequence(
            source,
            "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
            "approve",
            approve_args(source, contract, 1_000_000, 9_999).unwrap(),
            100,
        )
        .unwrap();
        let burn = encode_invoke_at_sequence(
            source,
            contract,
            "deposit_for_burn",
            deposit_for_burn_args(
                source,
                1_000_000,
                0,
                [1u8; 32],
                "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
                1,
                FINALITY_STANDARD,
            )
            .unwrap(),
            101,
        )
        .unwrap();
        assert_eq!(envelope_sequence(&approve).unwrap(), 100);
        assert_eq!(envelope_sequence(&burn).unwrap(), 101);
        assert_ne!(approve, burn);
    }
}
