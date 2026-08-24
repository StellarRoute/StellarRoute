//! Raw CCTP v2 message header/body parsing for corridor validation.
//!
//! Layout from Circle technical guide:
//! https://developers.circle.com/cctp/references/technical-guide#message-format

use thiserror::Error;

use crate::cctp::config::{FINALITY_FAST, FINALITY_STANDARD};

pub const MESSAGE_HEADER_LEN: usize = 148;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCctpMessage {
    pub version: u32,
    pub source_domain: u32,
    pub destination_domain: u32,
    pub nonce: [u8; 32],
    pub sender: [u8; 32],
    pub recipient: [u8; 32],
    pub destination_caller: [u8; 32],
    pub min_finality_threshold: u32,
    pub finality_threshold_executed: u32,
    pub body: ParsedBurnMessageBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBurnMessageBody {
    pub version: u32,
    pub burn_token: [u8; 32],
    pub mint_recipient: [u8; 32],
    pub amount: u128,
    pub message_sender: [u8; 32],
    pub max_fee: u128,
    pub fee_executed: u128,
    pub expiration_block: u128,
    pub hook_data: Vec<u8>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MessageParseError {
    #[error("too short")]
    TooShort,
    #[error("invalid hex")]
    InvalidHex,
    #[error("body too short")]
    BodyTooShort,
    #[error("overflow")]
    Overflow,
}

pub fn decode_hex_message(raw: &str) -> Result<Vec<u8>, MessageParseError> {
    let trimmed = raw.trim();
    let hex = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    hex::decode(hex).map_err(|_| MessageParseError::InvalidHex)
}

pub fn parse_cctp_v2_message(bytes: &[u8]) -> Result<ParsedCctpMessage, MessageParseError> {
    if bytes.len() < MESSAGE_HEADER_LEN {
        return Err(MessageParseError::TooShort);
    }
    let version = read_u32_be(bytes, 0)?;
    let source_domain = read_u32_be(bytes, 4)?;
    let destination_domain = read_u32_be(bytes, 8)?;
    let nonce = read_bytes32(bytes, 12)?;
    let sender = read_bytes32(bytes, 44)?;
    let recipient = read_bytes32(bytes, 76)?;
    let destination_caller = read_bytes32(bytes, 108)?;
    let min_finality_threshold = read_u32_be(bytes, 140)?;
    let finality_threshold_executed = read_u32_be(bytes, 144)?;
    let body_bytes = &bytes[148..];
    let body = parse_burn_message_body(body_bytes)?;

    Ok(ParsedCctpMessage {
        version,
        source_domain,
        destination_domain,
        nonce,
        sender,
        recipient,
        destination_caller,
        min_finality_threshold,
        finality_threshold_executed,
        body,
    })
}

pub fn parse_burn_message_body(bytes: &[u8]) -> Result<ParsedBurnMessageBody, MessageParseError> {
    if bytes.len() < 228 {
        return Err(MessageParseError::BodyTooShort);
    }
    let version = read_u32_be(bytes, 0)?;
    let burn_token = read_bytes32(bytes, 4)?;
    let mint_recipient = read_bytes32(bytes, 36)?;
    let amount = read_u256(bytes, 68)?;
    let message_sender = read_bytes32(bytes, 100)?;
    let max_fee = read_u256(bytes, 132)?;
    let fee_executed = read_u256(bytes, 164)?;
    let expiration_block = read_u256(bytes, 196)?;
    let hook_data = bytes[228..].to_vec();

    Ok(ParsedBurnMessageBody {
        version,
        burn_token,
        mint_recipient,
        amount,
        message_sender,
        max_fee,
        fee_executed,
        expiration_block,
        hook_data,
    })
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Result<u32, MessageParseError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(MessageParseError::TooShort)?;
    Ok(u32::from_be_bytes(slice.try_into().unwrap()))
}

fn read_bytes32(bytes: &[u8], offset: usize) -> Result<[u8; 32], MessageParseError> {
    let slice = bytes
        .get(offset..offset + 32)
        .ok_or(MessageParseError::TooShort)?;
    let mut out = [0u8; 32];
    out.copy_from_slice(slice);
    Ok(out)
}

fn read_u256(bytes: &[u8], offset: usize) -> Result<u128, MessageParseError> {
    let slice = bytes
        .get(offset..offset + 32)
        .ok_or(MessageParseError::TooShort)?;
    // CCTP amounts fit in u128 for testnet caps; reject high limbs.
    if slice[..16] != [0u8; 16] {
        return Err(MessageParseError::Overflow);
    }
    let mut limb = [0u8; 16];
    limb.copy_from_slice(&slice[16..32]);
    Ok(u128::from_be_bytes(limb))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorridorMessageExpectations {
    pub source_domain: u32,
    pub destination_domain: u32,
    pub header_recipient: [u8; 32],
    pub header_sender: [u8; 32],
    pub burn_token: [u8; 32],
    pub mint_recipient: [u8; 32],
    pub destination_caller: [u8; 32],
    pub amount_cctp_subunits: u128,
    pub min_finality: u32,
    pub body_message_sender: [u8; 32],
    pub hook_data: Option<Vec<u8>>,
    /// When true, hook_data must be empty (Stellar->EVM corridor).
    pub hook_data_required_empty: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MessageValidationError {
    #[error("parse: {0}")]
    Parse(MessageParseError),
    #[error("wrong source domain")]
    WrongSourceDomain,
    #[error("wrong destination domain")]
    WrongDestinationDomain,
    #[error("wrong header recipient")]
    WrongHeaderRecipient,
    #[error("wrong header sender")]
    WrongHeaderSender,
    #[error("wrong burn token")]
    WrongBurnToken,
    #[error("wrong mint recipient")]
    WrongMintRecipient,
    #[error("wrong destination caller")]
    WrongDestinationCaller,
    #[error("wrong amount")]
    WrongAmount,
    #[error("wrong finality")]
    WrongFinality,
    #[error("wrong message sender")]
    WrongMessageSender,
    #[error("wrong hook data")]
    WrongHookData,
    #[error("finality not executed")]
    FinalityNotExecuted,
    #[error("hook data too large")]
    HookDataTooLarge,
}

use crate::cctp::bounds::{check_byte_len, MAX_HOOK_DATA_BYTES, MAX_RAW_MESSAGE_BYTES};

pub fn validate_message_for_corridor(
    raw_hex: &str,
    expected: &CorridorMessageExpectations,
) -> Result<ParsedCctpMessage, MessageValidationError> {
    let bytes = decode_hex_message(raw_hex).map_err(MessageValidationError::Parse)?;
    check_byte_len("raw_message", &bytes, MAX_RAW_MESSAGE_BYTES)
        .map_err(|_| MessageValidationError::Parse(MessageParseError::TooShort))?;
    let parsed = parse_cctp_v2_message(&bytes).map_err(MessageValidationError::Parse)?;

    if parsed.source_domain != expected.source_domain {
        return Err(MessageValidationError::WrongSourceDomain);
    }
    if parsed.destination_domain != expected.destination_domain {
        return Err(MessageValidationError::WrongDestinationDomain);
    }
    if parsed.recipient != expected.header_recipient {
        return Err(MessageValidationError::WrongHeaderRecipient);
    }
    if parsed.sender != expected.header_sender {
        return Err(MessageValidationError::WrongHeaderSender);
    }
    if parsed.body.burn_token != expected.burn_token {
        return Err(MessageValidationError::WrongBurnToken);
    }
    if parsed.body.mint_recipient != expected.mint_recipient {
        return Err(MessageValidationError::WrongMintRecipient);
    }
    if parsed.body.message_sender != expected.body_message_sender {
        return Err(MessageValidationError::WrongMessageSender);
    }
    if parsed.destination_caller != expected.destination_caller {
        return Err(MessageValidationError::WrongDestinationCaller);
    }
    if parsed.body.amount != expected.amount_cctp_subunits {
        return Err(MessageValidationError::WrongAmount);
    }

    let normalized_min = normalize_finality(parsed.min_finality_threshold);
    let normalized_expected = normalize_finality(expected.min_finality);
    if normalized_min != normalized_expected {
        return Err(MessageValidationError::WrongFinality);
    }

    if parsed.finality_threshold_executed < normalized_min {
        return Err(MessageValidationError::FinalityNotExecuted);
    }

    if check_byte_len("hook_data", &parsed.body.hook_data, MAX_HOOK_DATA_BYTES).is_err() {
        return Err(MessageValidationError::HookDataTooLarge);
    }

    if expected.hook_data_required_empty {
        if !parsed.body.hook_data.is_empty() {
            return Err(MessageValidationError::WrongHookData);
        }
    } else if let Some(expected_hook) = &expected.hook_data {
        if parsed.body.hook_data != *expected_hook {
            return Err(MessageValidationError::WrongHookData);
        }
    } else if !parsed.body.hook_data.is_empty() {
        return Err(MessageValidationError::WrongHookData);
    }

    Ok(parsed)
}

pub fn normalize_finality(threshold: u32) -> u32 {
    if threshold <= FINALITY_FAST {
        FINALITY_FAST
    } else {
        FINALITY_STANDARD
    }
}

/// Build a synthetic v2 burn message for tests (deterministic layout).
pub fn build_synthetic_cctp_message(expected: &CorridorMessageExpectations) -> Vec<u8> {
    let hook_len = expected.hook_data.as_ref().map(|h| h.len()).unwrap_or(0);
    let mut msg = vec![0u8; 148 + 228 + hook_len];
    msg[4..8].copy_from_slice(&expected.source_domain.to_be_bytes());
    msg[8..12].copy_from_slice(&expected.destination_domain.to_be_bytes());
    msg[44..76].copy_from_slice(&expected.header_sender);
    msg[76..108].copy_from_slice(&expected.header_recipient);
    msg[108..140].copy_from_slice(&expected.destination_caller);
    msg[140..144].copy_from_slice(&expected.min_finality.to_be_bytes());
    msg[144..148].copy_from_slice(&expected.min_finality.to_be_bytes());

    let body_base = 148;
    msg[body_base + 4..body_base + 36].copy_from_slice(&expected.burn_token);
    msg[body_base + 36..body_base + 68].copy_from_slice(&expected.mint_recipient);
    msg[body_base + 68 + 16..body_base + 68 + 32]
        .copy_from_slice(&expected.amount_cctp_subunits.to_be_bytes());
    msg[body_base + 100..body_base + 132].copy_from_slice(&expected.body_message_sender);

    if hook_len > 0 {
        msg[body_base + 228..].copy_from_slice(expected.hook_data.as_ref().unwrap());
    }
    msg
}

pub fn encode_message_hex(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_synthetic_message() {
        let mut msg = vec![0u8; 148 + 228];
        msg[4..8].copy_from_slice(&27u32.to_be_bytes());
        msg[8..12].copy_from_slice(&0u32.to_be_bytes());
        msg[140..144].copy_from_slice(&FINALITY_STANDARD.to_be_bytes());
        msg[144..148].copy_from_slice(&FINALITY_STANDARD.to_be_bytes());
        let amount: u128 = 1000;
        msg[148 + 68 + 16..148 + 68 + 32].copy_from_slice(&amount.to_be_bytes());

        let parsed = parse_cctp_v2_message(&msg).unwrap();
        assert_eq!(parsed.source_domain, 27);
        assert_eq!(parsed.destination_domain, 0);
        assert_eq!(parsed.body.amount, 1000);
    }
}
