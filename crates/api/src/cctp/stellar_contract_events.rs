//! Decode Soroban `ContractEvent` XDR emitted by Circle CCTP v2 on Stellar Testnet.
//!
//! Event layouts pinned from `circlefin/stellar-cctp` @ `45746f2c8031`:
//! - `contracts/token-messenger-minter-v2/src/lib.rs` (`deposit_for_burn`)
//! - `contracts/message-transmitter-v2/src/lib.rs` (`message_sent`, `message_received`)
//! - `contracts/cctp-forwarder/src/lib.rs` (`mint_and_forward`)

use stellar_xdr::curr::{
    ContractEvent, ContractEventBody, Limits, ReadXdr, ScAddress, ScBytes, ScMap, ScMapEntry, ScVal,
};

use crate::cctp::bounds::{check_byte_len, MAX_RAW_MESSAGE_BYTES};
use crate::cctp::verifiers::VerifierError;

pub const MAX_CONTRACT_EVENTS_PER_TX: usize = 256;
pub const MAX_EVENTS_PER_OPERATION: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositForBurnEvent {
    pub burn_token: ScAddress,
    pub depositor: ScAddress,
    pub min_finality_threshold: u32,
    pub amount: i128,
    pub mint_recipient: [u8; 32],
    pub destination_domain: u32,
    pub destination_token_messenger: [u8; 32],
    pub destination_caller: [u8; 32],
    pub max_fee: i128,
    pub hook_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSentEvent {
    pub message: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageReceivedEvent {
    pub caller: ScAddress,
    pub nonce: [u8; 32],
    pub finality_threshold_executed: u32,
    pub source_domain: u32,
    pub sender: [u8; 32],
    pub message_body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintAndForwardEvent {
    pub forward_recipient: String,
    pub token: ScAddress,
    pub amount: i128,
}

pub fn decode_contract_event_xdr(b64: &str) -> Result<ContractEvent, VerifierError> {
    ContractEvent::from_xdr_base64(b64, Limits::none())
        .map_err(|e| VerifierError::Failed(e.to_string()))
}

pub fn event_v0(event: &ContractEvent) -> Result<(&[ScVal], &ScVal), VerifierError> {
    match &event.body {
        ContractEventBody::V0(v0) => Ok((v0.topics.as_slice(), &v0.data)),
    }
}

pub fn topic_symbol(topics: &[ScVal], index: usize) -> Result<String, VerifierError> {
    match topics.get(index) {
        Some(ScVal::Symbol(sym)) => Ok(sym.0.to_string()),
        _ => Err(VerifierError::Failed("missing event topic symbol".into())),
    }
}

pub fn scval_to_address(val: &ScVal) -> Result<ScAddress, VerifierError> {
    match val {
        ScVal::Address(a) => Ok(a.clone()),
        _ => Err(VerifierError::Failed("expected address".into())),
    }
}

pub fn scval_to_i128(val: &ScVal) -> Result<i128, VerifierError> {
    match val {
        ScVal::I128(parts) => {
            let hi = parts.hi as i128;
            let lo = parts.lo as i128;
            Ok((hi << 64) | lo)
        }
        _ => Err(VerifierError::Failed("expected i128".into())),
    }
}

pub fn scval_to_u32(val: &ScVal) -> Result<u32, VerifierError> {
    match val {
        ScVal::U32(v) => Ok(*v),
        _ => Err(VerifierError::Failed("expected u32".into())),
    }
}

pub fn scval_to_bytes32(val: &ScVal) -> Result<[u8; 32], VerifierError> {
    match val {
        ScVal::Bytes(ScBytes(bytes)) if bytes.len() == 32 => {
            let mut out = [0u8; 32];
            out.copy_from_slice(bytes);
            Ok(out)
        }
        _ => Err(VerifierError::Failed("expected bytes32".into())),
    }
}

pub fn scval_to_bytes(val: &ScVal) -> Result<Vec<u8>, VerifierError> {
    match val {
        ScVal::Bytes(ScBytes(bytes)) => Ok(bytes.to_vec()),
        _ => Err(VerifierError::Failed("expected bytes".into())),
    }
}

pub fn map_get<'a>(data: &'a ScVal, key: &str) -> Result<&'a ScVal, VerifierError> {
    let ScVal::Map(Some(ScMap(entries))) = data else {
        return Err(VerifierError::Failed("expected event map".into()));
    };
    for entry in entries.iter() {
        let ScMapEntry { key: map_key, val } = entry;
        if let ScVal::Symbol(sym) = map_key {
            if sym.0.to_string() == key {
                return Ok(val);
            }
        }
    }
    Err(VerifierError::Failed(format!("missing field {key}")))
}

pub fn address_to_strkey(addr: &ScAddress) -> Result<String, VerifierError> {
    match addr {
        ScAddress::Contract(contract) => Ok(format!("{}", stellar_strkey::Contract(contract.0 .0))),
        ScAddress::Account(account_id) => {
            use stellar_xdr::curr::{PublicKey, Uint256};
            let PublicKey::PublicKeyTypeEd25519(Uint256(bytes)) = account_id.0;
            Ok(format!("{}", stellar_strkey::ed25519::PublicKey(bytes)))
        }
        ScAddress::MuxedAccount(muxed) => Ok(format!(
            "{}",
            stellar_strkey::ed25519::MuxedAccount {
                ed25519: muxed.ed25519.0,
                id: muxed.id,
            }
        )),
        ScAddress::ClaimableBalance(_) | ScAddress::LiquidityPool(_) => {
            Err(VerifierError::Failed("unsupported address type".into()))
        }
    }
}

pub fn contract_hash(event: &ContractEvent) -> Result<[u8; 32], VerifierError> {
    let hash = event
        .contract_id
        .as_ref()
        .ok_or_else(|| VerifierError::Failed("missing contract id".into()))?;
    Ok(hash.0 .0)
}

pub fn parse_deposit_for_burn(event: &ContractEvent) -> Result<DepositForBurnEvent, VerifierError> {
    let (topics, data) = event_v0(event)?;
    if topic_symbol(topics, 0)? != "deposit_for_burn" {
        return Err(VerifierError::Failed("wrong event".into()));
    }
    if topics.len() != 4 {
        return Err(VerifierError::Failed("deposit_for_burn topic count".into()));
    }
    Ok(DepositForBurnEvent {
        burn_token: scval_to_address(&topics[1])?,
        depositor: scval_to_address(&topics[2])?,
        min_finality_threshold: scval_to_u32(&topics[3])?,
        amount: scval_to_i128(map_get(data, "amount")?)?,
        mint_recipient: scval_to_bytes32(map_get(data, "mint_recipient")?)?,
        destination_domain: scval_to_u32(map_get(data, "destination_domain")?)?,
        destination_token_messenger: scval_to_bytes32(map_get(
            data,
            "destination_token_messenger",
        )?)?,
        destination_caller: scval_to_bytes32(map_get(data, "destination_caller")?)?,
        max_fee: scval_to_i128(map_get(data, "max_fee")?)?,
        hook_data: match map_get(data, "hook_data") {
            Ok(ScVal::Void) => Vec::new(),
            Ok(v) => scval_to_bytes(v)?,
            Err(_) => Vec::new(),
        },
    })
}

pub fn parse_message_sent(event: &ContractEvent) -> Result<MessageSentEvent, VerifierError> {
    let (topics, data) = event_v0(event)?;
    if topic_symbol(topics, 0)? != "message_sent" {
        return Err(VerifierError::Failed("wrong event".into()));
    }
    if topics.len() != 1 {
        return Err(VerifierError::Failed("message_sent topic count".into()));
    }
    let message = scval_to_bytes(map_get(data, "message")?)?;
    check_byte_len("message", &message, MAX_RAW_MESSAGE_BYTES).map_err(VerifierError::Failed)?;
    Ok(MessageSentEvent { message })
}

pub fn parse_message_received(
    event: &ContractEvent,
) -> Result<MessageReceivedEvent, VerifierError> {
    let (topics, data) = event_v0(event)?;
    if topic_symbol(topics, 0)? != "message_received" {
        return Err(VerifierError::Failed("wrong event".into()));
    }
    if topics.len() != 4 {
        return Err(VerifierError::Failed("message_received topic count".into()));
    }
    let message_body = scval_to_bytes(map_get(data, "message_body")?)?;
    check_byte_len("message_body", &message_body, MAX_RAW_MESSAGE_BYTES)
        .map_err(VerifierError::Failed)?;
    Ok(MessageReceivedEvent {
        caller: scval_to_address(&topics[1])?,
        nonce: scval_to_bytes32(&topics[2])?,
        finality_threshold_executed: scval_to_u32(&topics[3])?,
        source_domain: scval_to_u32(map_get(data, "source_domain")?)?,
        sender: scval_to_bytes32(map_get(data, "sender")?)?,
        message_body,
    })
}

pub fn parse_mint_and_forward(event: &ContractEvent) -> Result<MintAndForwardEvent, VerifierError> {
    let (topics, data) = event_v0(event)?;
    if topic_symbol(topics, 0)? != "mint_and_forward" {
        return Err(VerifierError::Failed("wrong event".into()));
    }
    if topics.len() != 1 {
        return Err(VerifierError::Failed("mint_and_forward topic count".into()));
    }
    let forward_recipient = map_get(data, "forward_recipient")?;
    let recipient_str = crate::cctp::stellar_muxed::muxed_recipient_from_scval(forward_recipient)?;
    Ok(MintAndForwardEvent {
        forward_recipient: recipient_str,
        token: scval_to_address(map_get(data, "token")?)?,
        amount: scval_to_i128(map_get(data, "amount")?)?,
    })
}

pub fn collect_contract_events(
    nested: &[Vec<String>],
) -> Result<Vec<ContractEvent>, VerifierError> {
    if nested.len() > MAX_CONTRACT_EVENTS_PER_TX {
        return Err(VerifierError::Failed(
            "too many operation event groups".into(),
        ));
    }
    let mut out = Vec::new();
    for group in nested {
        if group.len() > MAX_EVENTS_PER_OPERATION {
            return Err(VerifierError::Failed("too many contract events".into()));
        }
        for b64 in group {
            if b64.len() > 256 * 1024 {
                return Err(VerifierError::Failed("event xdr too large".into()));
            }
            out.push(decode_contract_event_xdr(b64)?);
        }
    }
    if out.len() > MAX_CONTRACT_EVENTS_PER_TX {
        return Err(VerifierError::Failed("too many contract events".into()));
    }
    Ok(out)
}

#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use stellar_xdr::curr::{
        ContractEventType, ContractId, ExtensionPoint, Hash, ScSymbol, VecM, WriteXdr,
    };

    pub fn build_event_map(fields: Vec<(&str, ScVal)>) -> ScVal {
        let entries: Vec<ScMapEntry> = fields
            .into_iter()
            .map(|(k, v)| ScMapEntry {
                key: ScVal::Symbol(ScSymbol::try_from(k.to_string()).unwrap()),
                val: v,
            })
            .collect();
        ScVal::Map(Some(ScMap(entries.try_into().unwrap())))
    }

    pub fn deposit_for_burn_event(
        contract_hash: [u8; 32],
        burn_token: ScAddress,
        depositor: ScAddress,
        min_finality: u32,
        data: ScVal,
    ) -> ContractEvent {
        ContractEvent {
            ext: ExtensionPoint::V0,
            contract_id: Some(ContractId(Hash(contract_hash))),
            type_: ContractEventType::Contract,
            body: ContractEventBody::V0(stellar_xdr::curr::ContractEventV0 {
                topics: vec![
                    ScVal::Symbol(ScSymbol::try_from("deposit_for_burn".to_string()).unwrap()),
                    ScVal::Address(burn_token),
                    ScVal::Address(depositor),
                    ScVal::U32(min_finality),
                ]
                .try_into()
                .unwrap(),
                data,
            }),
        }
    }

    pub fn message_sent_event(contract_hash: [u8; 32], message: Vec<u8>) -> ContractEvent {
        ContractEvent {
            ext: ExtensionPoint::V0,
            contract_id: Some(ContractId(Hash(contract_hash))),
            type_: ContractEventType::Contract,
            body: ContractEventBody::V0(stellar_xdr::curr::ContractEventV0 {
                topics: vec![ScVal::Symbol(
                    ScSymbol::try_from("message_sent".to_string()).unwrap(),
                )]
                .try_into()
                .unwrap(),
                data: build_event_map(vec![(
                    "message",
                    ScVal::Bytes(ScBytes(message.try_into().unwrap())),
                )]),
            }),
        }
    }

    pub fn mint_and_forward_event(
        contract_hash: [u8; 32],
        forward_recipient: ScVal,
        token: ScAddress,
        amount: i128,
    ) -> ContractEvent {
        ContractEvent {
            ext: ExtensionPoint::V0,
            contract_id: Some(ContractId(Hash(contract_hash))),
            type_: ContractEventType::Contract,
            body: ContractEventBody::V0(stellar_xdr::curr::ContractEventV0 {
                topics: vec![ScVal::Symbol(
                    ScSymbol::try_from("mint_and_forward".to_string()).unwrap(),
                )]
                .try_into()
                .unwrap(),
                data: build_event_map(vec![
                    ("forward_recipient", forward_recipient),
                    ("token", ScVal::Address(token)),
                    (
                        "amount",
                        ScVal::I128(stellar_xdr::curr::Int128Parts {
                            hi: (amount >> 64) as i64,
                            lo: amount as u64,
                        }),
                    ),
                ]),
            }),
        }
    }

    pub fn event_to_b64(event: &ContractEvent) -> String {
        event
            .to_xdr_base64(Limits::none())
            .expect("event xdr encode")
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::*;
    use super::*;
    use crate::cctp::builders::stellar::encoder::{account_address, contract_address};
    use crate::cctp::config::{STELLAR_TOKEN_MESSENGER, STELLAR_USDC_CONTRACT};

    #[test]
    fn roundtrip_deposit_for_burn_event() {
        let contract = stellar_strkey::Contract::from_string(STELLAR_TOKEN_MESSENGER).unwrap();
        let burn = contract_address(STELLAR_USDC_CONTRACT).unwrap();
        let depositor =
            account_address("GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF").unwrap();
        let data = build_event_map(vec![
            (
                "amount",
                ScVal::I128(stellar_xdr::curr::Int128Parts {
                    hi: 0,
                    lo: 1_000_000,
                }),
            ),
            (
                "mint_recipient",
                ScVal::Bytes(ScBytes([1u8; 32].to_vec().try_into().unwrap())),
            ),
            ("destination_domain", ScVal::U32(0)),
            (
                "destination_token_messenger",
                ScVal::Bytes(ScBytes([2u8; 32].to_vec().try_into().unwrap())),
            ),
            (
                "destination_caller",
                ScVal::Bytes(ScBytes([0u8; 32].to_vec().try_into().unwrap())),
            ),
            (
                "max_fee",
                ScVal::I128(stellar_xdr::curr::Int128Parts { hi: 0, lo: 1 }),
            ),
            ("hook_data", ScVal::Void),
        ]);
        let event = deposit_for_burn_event(contract.0, burn, depositor, 2000, data);
        let parsed = parse_deposit_for_burn(&event).unwrap();
        assert_eq!(parsed.amount, 1_000_000);
        assert_eq!(parsed.destination_domain, 0);
    }
}
