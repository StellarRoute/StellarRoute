//! Direction-correct CCTP v2 message expectations for the first testnet corridor.
//!
//! Sources:
//! - https://developers.circle.com/cctp/references/stellar (forwarder mintRecipient/destinationCaller/hook)
//! - https://developers.circle.com/cctp/references/technical-guide (message layout, bytes32(0) caller)

use thiserror::Error;

use crate::cctp::config::{
    corridor_min_finality, CctpConfig, SEPOLIA_DOMAIN, SEPOLIA_TOKEN_MESSENGER, SEPOLIA_USDC,
    STELLAR_CCTP_FORWARDER, STELLAR_TESTNET_DOMAIN,
};
use crate::cctp::encoding::{
    build_forwarder_hook_data_recipient, decimal_to_cctp_subunits, evm_address_to_bytes32,
    stellar_account_to_bytes32, stellar_contract_to_bytes32, stellar_outbound_cctp_amount,
};
use crate::cctp::message::CorridorMessageExpectations;
use crate::cctp::store::CctpTransfer;
use crate::models::v2_cctp::{CctpDirection, SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID};

/// bytes32(0) — any address may call `receive_message` on destination (Circle technical guide).
pub const ANY_DESTINATION_CALLER: [u8; 32] = [0u8; 32];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExpectationError {
    #[error("encoding: {0}")]
    Encoding(String),
    #[error("unsupported direction")]
    UnsupportedDirection,
}

fn body_message_sender(
    transfer: &CctpTransfer,
    direction: CctpDirection,
) -> Result<[u8; 32], ExpectationError> {
    match direction {
        CctpDirection::StellarToEvm => {
            if transfer.sender.is_empty() {
                Ok([0u8; 32])
            } else {
                stellar_account_to_bytes32(&transfer.sender)
                    .map_err(|e| ExpectationError::Encoding(e.to_string()))
            }
        }
        CctpDirection::EvmToStellar => {
            if transfer.sender.is_empty() {
                Ok([0u8; 32])
            } else {
                evm_address_to_bytes32(&transfer.sender)
                    .map_err(|e| ExpectationError::Encoding(e.to_string()))
            }
        }
    }
}

pub fn build_corridor_expectations(
    transfer: &CctpTransfer,
    config: &CctpConfig,
) -> Result<CorridorMessageExpectations, ExpectationError> {
    let amount_cctp = amount_cctp_subunits(transfer)?;

    match transfer.direction {
        CctpDirection::StellarToEvm => {
            let burn_token = stellar_contract_to_bytes32(&config.contracts.stellar_usdc)
                .map_err(|e| ExpectationError::Encoding(e.to_string()))?;
            let mint_recipient = evm_address_to_bytes32(&transfer.recipient)
                .map_err(|e| ExpectationError::Encoding(e.to_string()))?;
            let body_sender = body_message_sender(transfer, CctpDirection::StellarToEvm)?;
            Ok(CorridorMessageExpectations {
                source_domain: STELLAR_TESTNET_DOMAIN,
                destination_domain: SEPOLIA_DOMAIN,
                header_recipient: evm_address_to_bytes32(SEPOLIA_TOKEN_MESSENGER)
                    .map_err(|e| ExpectationError::Encoding(e.to_string()))?,
                header_sender: stellar_contract_to_bytes32(
                    &config.contracts.stellar_token_messenger,
                )
                .map_err(|e| ExpectationError::Encoding(e.to_string()))?,
                burn_token,
                mint_recipient,
                destination_caller: ANY_DESTINATION_CALLER,
                amount_cctp_subunits: amount_cctp,
                min_finality: corridor_min_finality(transfer.finality),
                body_message_sender: body_sender,
                hook_data: None,
                hook_data_required_empty: true,
            })
        }
        CctpDirection::EvmToStellar => {
            let burn_token = evm_address_to_bytes32(SEPOLIA_USDC)
                .map_err(|e| ExpectationError::Encoding(e.to_string()))?;
            let forwarder = stellar_contract_to_bytes32(STELLAR_CCTP_FORWARDER)
                .map_err(|e| ExpectationError::Encoding(e.to_string()))?;
            let hook = build_forwarder_hook_data_recipient(&transfer.recipient)
                .map_err(|e| ExpectationError::Encoding(e.to_string()))?;
            let body_sender = body_message_sender(transfer, CctpDirection::EvmToStellar)?;
            Ok(CorridorMessageExpectations {
                source_domain: SEPOLIA_DOMAIN,
                destination_domain: STELLAR_TESTNET_DOMAIN,
                // Circle inbound-to-Stellar: header recipient is TokenMessengerMinterV2
                // (same role as SEPOLIA_TOKEN_MESSENGER on the reverse corridor).
                header_recipient: stellar_contract_to_bytes32(
                    &config.contracts.stellar_token_messenger,
                )
                .map_err(|e| ExpectationError::Encoding(e.to_string()))?,
                header_sender: evm_address_to_bytes32(SEPOLIA_TOKEN_MESSENGER)
                    .map_err(|e| ExpectationError::Encoding(e.to_string()))?,
                burn_token,
                mint_recipient: forwarder,
                destination_caller: forwarder,
                amount_cctp_subunits: amount_cctp,
                min_finality: corridor_min_finality(transfer.finality),
                body_message_sender: body_sender,
                hook_data: Some(hook),
                hook_data_required_empty: false,
            })
        }
    }
}

pub fn build_expected_burn_facts(
    transfer: &CctpTransfer,
    config: &CctpConfig,
    tx_hash: &str,
) -> Result<crate::cctp::verifiers::VerifiedBurnFacts, ExpectationError> {
    let amount_cctp = amount_cctp_subunits(transfer)?;
    let expectations = build_corridor_expectations(transfer, config)?;

    let (source_chain, source_domain, dest_domain) = match transfer.direction {
        CctpDirection::StellarToEvm => (
            STELLAR_TESTNET_CHAIN_ID,
            STELLAR_TESTNET_DOMAIN,
            SEPOLIA_DOMAIN,
        ),
        CctpDirection::EvmToStellar => (SEPOLIA_CHAIN_ID, SEPOLIA_DOMAIN, STELLAR_TESTNET_DOMAIN),
    };

    let token_messenger = match transfer.direction {
        CctpDirection::StellarToEvm => {
            stellar_contract_to_bytes32(&config.contracts.stellar_token_messenger)
                .map_err(|e| ExpectationError::Encoding(e.to_string()))?
        }
        CctpDirection::EvmToStellar => evm_address_to_bytes32(SEPOLIA_TOKEN_MESSENGER)
            .map_err(|e| ExpectationError::Encoding(e.to_string()))?,
    };

    Ok(crate::cctp::verifiers::VerifiedBurnFacts {
        tx_hash: tx_hash.to_string(),
        source_chain_id: source_chain.into(),
        source_domain,
        destination_domain: dest_domain,
        sender: transfer.sender.clone(),
        amount_cctp_subunits: amount_cctp,
        burn_token_bytes32: expectations.burn_token,
        mint_recipient_bytes32: expectations.mint_recipient,
        destination_caller_bytes32: expectations.destination_caller,
        min_finality_threshold: corridor_min_finality(transfer.finality),
        hook_data: expectations.hook_data,
        token_messenger_bytes32: token_messenger,
        block_or_ledger: None,
    })
}

fn amount_cctp_subunits(transfer: &CctpTransfer) -> Result<u128, ExpectationError> {
    match transfer.direction {
        CctpDirection::StellarToEvm => {
            let (amt, _) = stellar_outbound_cctp_amount(&transfer.amount)
                .map_err(|e| ExpectationError::Encoding(e.to_string()))?;
            Ok(amt)
        }
        CctpDirection::EvmToStellar => decimal_to_cctp_subunits(&transfer.amount)
            .map_err(|e| ExpectationError::Encoding(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::config::{
        CctpConfig, STELLAR_CCTP_FORWARDER, STELLAR_TOKEN_MESSENGER, STELLAR_USDC_CONTRACT,
    };
    use crate::cctp::store::CctpTransfer;
    use crate::models::v2_cctp::{CctpDirection, CctpFinality, CctpTransferStatus};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    const G_RECIPIENT: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
    const EVM_RECIPIENT: &str = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";

    fn base_transfer(direction: CctpDirection, recipient: &str) -> CctpTransfer {
        let now = Utc::now();
        CctpTransfer {
            transfer_id: Uuid::new_v4(),
            support_reference_id: "sup".into(),
            corridor_id: "c".into(),
            provider: "circle-cctp".into(),
            direction,
            source_chain_id: if direction == CctpDirection::StellarToEvm {
                STELLAR_TESTNET_CHAIN_ID.into()
            } else {
                SEPOLIA_CHAIN_ID.into()
            },
            destination_chain_id: if direction == CctpDirection::StellarToEvm {
                SEPOLIA_CHAIN_ID.into()
            } else {
                STELLAR_TESTNET_CHAIN_ID.into()
            },
            source_asset: "a".into(),
            source_asset_canonical: "a".into(),
            destination_asset: "b".into(),
            destination_asset_canonical: "b".into(),
            sender: "".into(),
            recipient: recipient.into(),
            mint_submitter: None,
            amount: "100.000000".into(),
            destination_amount: "100.000000".into(),
            finality: CctpFinality::Standard,
            runtime_fee_quote: None,
            max_fee: None,
            fee_expires_at: None,
            quote_expires_at: now + Duration::minutes(5),
            status: CctpTransferStatus::Created,
            source_tx_hash: None,
            source_approval_tx_hash: None,
            source_approval_verified_at: None,
            destination_tx_hash: None,
            iris_message_hash: None,
            message_nonce: None,
            raw_message: None,
            attestation: None,
            retry_count: 0,
            last_provider_error: None,
            last_provider_code: None,
            version: 1,
            created_at: now,
            updated_at: now,
            terminal_at: None,
            mint_payload_hash: None,
            mint_payload_expires_at: None,
            approval_payload_hash: None,
            approval_expiration_ledger: None,
            burn_payload_hash: None,
            burn_prepare_step: None,
            access_token_hash: None,
            last_polled_at: None,
            poll_lease_until: None,
            reattest_lease_owner_hash: None,
            reattest_lease_until: None,
            reattest_attempt_count: 0,
            reattest_cooldown_until: None,
        }
    }

    #[test]
    fn stellar_to_evm_expects_evm_mint_recipient_and_any_caller() {
        let cfg = CctpConfig::default_testnet();
        let transfer = base_transfer(CctpDirection::StellarToEvm, EVM_RECIPIENT);
        let exp = build_corridor_expectations(&transfer, &cfg).unwrap();
        assert_eq!(exp.source_domain, 27);
        assert_eq!(exp.destination_domain, 0);
        assert_eq!(exp.destination_caller, ANY_DESTINATION_CALLER);
        assert!(exp.hook_data.is_none());
        assert_eq!(
            exp.mint_recipient,
            evm_address_to_bytes32(EVM_RECIPIENT).unwrap()
        );
        assert_eq!(
            exp.burn_token,
            stellar_contract_to_bytes32(STELLAR_USDC_CONTRACT).unwrap()
        );
    }

    #[test]
    fn evm_to_stellar_expects_forwarder_and_hook() {
        let cfg = CctpConfig::default_testnet();
        let transfer = base_transfer(CctpDirection::EvmToStellar, G_RECIPIENT);
        let exp = build_corridor_expectations(&transfer, &cfg).unwrap();
        let forwarder = stellar_contract_to_bytes32(STELLAR_CCTP_FORWARDER).unwrap();
        assert_eq!(exp.mint_recipient, forwarder);
        assert_eq!(exp.destination_caller, forwarder);
        assert_eq!(
            exp.header_recipient,
            stellar_contract_to_bytes32(STELLAR_TOKEN_MESSENGER).unwrap()
        );
        assert_eq!(
            exp.hook_data,
            Some(build_forwarder_hook_data_recipient(G_RECIPIENT).unwrap())
        );
        assert_eq!(
            exp.burn_token,
            evm_address_to_bytes32(SEPOLIA_USDC).unwrap()
        );
        assert_eq!(
            exp.min_finality,
            corridor_min_finality(CctpFinality::Standard)
        );
    }

    #[test]
    fn evm_to_stellar_fast_uses_fast_finality_threshold() {
        let cfg = CctpConfig::default_testnet();
        let mut transfer = base_transfer(CctpDirection::EvmToStellar, G_RECIPIENT);
        transfer.finality = CctpFinality::Fast;
        let exp = build_corridor_expectations(&transfer, &cfg).unwrap();
        assert_eq!(exp.min_finality, corridor_min_finality(CctpFinality::Fast));
        let facts = build_expected_burn_facts(&transfer, &cfg, "0xabc").unwrap();
        assert_eq!(
            facts.min_finality_threshold,
            corridor_min_finality(CctpFinality::Fast)
        );
    }

    #[test]
    fn stellar_to_evm_fast_uses_fast_finality_threshold() {
        let cfg = CctpConfig::default_testnet();
        let mut transfer = base_transfer(CctpDirection::StellarToEvm, EVM_RECIPIENT);
        transfer.finality = CctpFinality::Fast;
        let exp = build_corridor_expectations(&transfer, &cfg).unwrap();
        assert_eq!(exp.min_finality, corridor_min_finality(CctpFinality::Fast));
        let facts = build_expected_burn_facts(&transfer, &cfg, "0xabc").unwrap();
        assert_eq!(
            facts.min_finality_threshold,
            corridor_min_finality(CctpFinality::Fast)
        );
    }

    #[test]
    fn live_iris_fast_sepolia_burn_validates_against_corridor() {
        use crate::cctp::message::validate_message_for_corridor;

        let cfg = CctpConfig::default_testnet();
        let mut transfer = base_transfer(
            CctpDirection::EvmToStellar,
            "GBSTOZPBODWWNR4LIX56BH5IGGDHABGO43YGCZOSMVDVXMOZOHZIN5YI",
        );
        transfer.sender = "0xa632da1e4d5dd7fb236a0b798ff9331e3a9930df".into();
        transfer.amount = "20".into();
        transfer.destination_amount = "20".into();
        transfer.finality = CctpFinality::Fast;
        let exp = build_corridor_expectations(&transfer, &cfg).unwrap();
        let msg = include_str!("testdata/iris_fast_bb03103d.message.hex").trim();
        validate_message_for_corridor(msg, &exp).expect("corridor validation should pass");
    }
}
