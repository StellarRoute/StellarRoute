//! Circle Iris v2 HTTP client for CCTP attestation polling.

use std::time::Duration;

use async_trait::async_trait;
use reqwest::redirect::Policy;
use serde::Deserialize;
use thiserror::Error;

use crate::cctp::bounds::{
    check_str_len, MAX_ATTESTATION_BYTES, MAX_IRIS_JSON_BYTES, MAX_RAW_MESSAGE_BYTES,
    MAX_TX_HASH_LEN,
};
use crate::cctp::config::{
    corridor_min_finality, parse_service_url, redact_url, CctpConfig, IRIS_SANDBOX_HOST,
    STELLAR_TESTNET_DOMAIN,
};
use crate::models::v2_cctp::CctpFinality;

const USER_AGENT: &str = "stellarroute-api/cctp-core/1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrisFeeQuote {
    pub standard_fee: String,
    pub fast_fee: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrisMessage {
    pub message_hex: String,
    pub attestation_hex: Option<String>,
    pub cctp_version: u32,
    pub status: IrisMessageStatus,
    pub event_nonce: String,
    pub source_tx_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrisMessageStatus {
    Pending,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrisPollOutcome {
    Pending,
    Complete(IrisMessage),
    RateLimited { retry_after_secs: u64 },
    NotFound,
}

#[derive(Debug, Error)]
pub enum IrisError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("timeout")]
    Timeout,
    #[error("malformed response: {0}")]
    Malformed(String),
    #[error("redirect blocked")]
    RedirectBlocked,
    #[error("host not allowlisted")]
    HostNotAllowlisted,
    #[error("response too large")]
    ResponseTooLarge,
}

#[async_trait]
pub trait IrisClient: Send + Sync {
    async fn fetch_burn_fees(
        &self,
        source_domain: u32,
        dest_domain: u32,
    ) -> Result<IrisFeeQuote, IrisError>;

    async fn poll_messages_by_tx(
        &self,
        source_domain: u32,
        tx_hash: &str,
    ) -> Result<IrisPollOutcome, IrisError>;

    async fn reattest(&self, nonce: &str) -> Result<(), IrisError>;
}

pub struct ReqwestIrisClient {
    client: reqwest::Client,
    config: CctpConfig,
    base_url: String,
    allowed_host: String,
    max_retries: u32,
}

impl ReqwestIrisClient {
    pub fn from_config(config: &CctpConfig) -> Result<Self, IrisError> {
        let parsed = parse_service_url(&config.iris_base_url)
            .map_err(|e| IrisError::Malformed(e.to_string()))?;
        let allowed_host = if cfg!(test) && is_local_test_host(&parsed.host) {
            parsed.host.clone()
        } else {
            config
                .validate()
                .map_err(|e| IrisError::Malformed(e.to_string()))?;
            IRIS_SANDBOX_HOST.to_string()
        };
        let base_url = config.iris_base_url.trim_end_matches('/').to_string();

        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(Policy::none())
            .timeout(Duration::from_secs(config.iris_timeout_secs))
            .build()
            .map_err(|e| IrisError::Http(e.to_string()))?;

        Ok(Self {
            client,
            config: config.clone(),
            base_url,
            allowed_host,
            max_retries: config.iris_max_retries,
        })
    }

    fn ensure_host(&self, url: &str) -> Result<(), IrisError> {
        if cfg!(test) {
            let parsed = parse_service_url(url).map_err(|_| IrisError::HostNotAllowlisted)?;
            if is_local_test_host(&parsed.host) && parsed.scheme == "http" {
                return Ok(());
            }
        }
        self.config
            .request_url_matches_allowed_host(url, &self.allowed_host)
            .map_err(|_| IrisError::HostNotAllowlisted)
    }

    async fn get_with_retries(&self, url: &str) -> Result<reqwest::Response, IrisError> {
        self.ensure_host(url)?;
        let mut attempt = 0;
        loop {
            let response = self.client.get(url).send().await;
            match response {
                Ok(resp) => return Ok(resp),
                Err(e) if e.is_timeout() => return Err(IrisError::Timeout),
                Err(e) if e.is_redirect() => return Err(IrisError::RedirectBlocked),
                Err(_e) if attempt < self.max_retries => {
                    attempt += 1;
                    tokio::time::sleep(Duration::from_millis(50 + attempt as u64 * 25)).await;
                    continue;
                }
                Err(e) => return Err(IrisError::Http(redact_url(&e.to_string()))),
            }
        }
    }

    async fn read_bounded_json<T: for<'de> serde::Deserialize<'de>>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, IrisError> {
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| IrisError::Http(redact_url(&e.to_string())))?;
        if bytes.len() > MAX_IRIS_JSON_BYTES {
            return Err(IrisError::ResponseTooLarge);
        }
        serde_json::from_slice(&bytes).map_err(|e| IrisError::Malformed(e.to_string()))
    }
}

fn is_local_test_host(host: &str) -> bool {
    host == "127.0.0.1" || host == "localhost"
}

#[derive(Debug, Deserialize)]
pub(crate) struct FeeTierResponse {
    #[serde(rename = "finalityThreshold")]
    finality_threshold: u32,
    #[serde(rename = "minimumFee")]
    minimum_fee: u64,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    messages: Vec<MessageV2>,
    #[serde(default, rename = "sourceTxHash")]
    source_tx_hash: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct MessageV2 {
    /// Iris returns `null` while status is `pending_confirmations`.
    #[serde(default)]
    message: Option<String>,
    attestation: Option<String>,
    #[serde(rename = "cctpVersion")]
    cctp_version: Option<u32>,
    status: Option<String>,
    #[serde(rename = "eventNonce")]
    event_nonce: Option<String>,
}

fn validate_minimum_fee(fee: u64) -> Result<(), IrisError> {
    if fee > 1_000_000_000_000 {
        return Err(IrisError::Malformed("minimumFee out of bounds".into()));
    }
    Ok(())
}

pub(crate) fn parse_burn_fee_tiers(tiers: Vec<FeeTierResponse>) -> Result<IrisFeeQuote, IrisError> {
    use std::collections::HashSet;

    let standard_threshold = corridor_min_finality(CctpFinality::Standard);
    let fast_threshold = corridor_min_finality(CctpFinality::Fast);
    let mut seen = HashSet::new();
    let mut standard_fee = None;
    let mut fast_fee = None;

    for tier in tiers {
        if !seen.insert(tier.finality_threshold) {
            return Err(IrisError::Malformed("duplicate finality tier".into()));
        }
        validate_minimum_fee(tier.minimum_fee)?;
        if tier.finality_threshold == standard_threshold {
            if standard_fee.is_some() {
                return Err(IrisError::Malformed("duplicate standard tier".into()));
            }
            standard_fee = Some(tier.minimum_fee.to_string());
        } else if tier.finality_threshold == fast_threshold {
            fast_fee = Some(tier.minimum_fee.to_string());
        } else {
            return Err(IrisError::Malformed("unknown finality tier".into()));
        }
    }

    let standard_fee =
        standard_fee.ok_or_else(|| IrisError::Malformed("missing standard tier".into()))?;
    Ok(IrisFeeQuote {
        standard_fee,
        fast_fee,
    })
}

pub fn normalize_tx_hash(hash: &str) -> String {
    let trimmed = hash.trim();
    let hex = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    hex.to_ascii_lowercase()
}

/// Iris `transactionHash` query form is domain-specific.
///
/// - EVM domains: lowercase `0x…` (sandbox rejects mixed-case / bare hex).
/// - Stellar domain 27: bare lowercase hex — `0x…` returns "Message not found".
pub fn iris_query_tx_hash(source_domain: u32, tx_hash: &str) -> String {
    let hex = normalize_tx_hash(tx_hash);
    if source_domain == STELLAR_TESTNET_DOMAIN {
        hex
    } else {
        format!("0x{hex}")
    }
}

pub(crate) fn select_complete_v2_message(
    messages: &[MessageV2],
    expected_tx_hash: &str,
    response_tx_hash: Option<&str>,
) -> Result<MessageV2, IrisError> {
    check_str_len("tx_hash", expected_tx_hash, MAX_TX_HASH_LEN).map_err(IrisError::Malformed)?;

    if let Some(resp_hash) = response_tx_hash {
        if normalize_tx_hash(resp_hash) != normalize_tx_hash(expected_tx_hash) {
            return Err(IrisError::Malformed("sourceTxHash mismatch".into()));
        }
    }

    let mut complete: Vec<&MessageV2> = Vec::new();
    for msg in messages {
        let cctp_version = msg.cctp_version.unwrap_or(0);
        if cctp_version != 2 {
            continue;
        }
        let status = msg.status.as_deref();
        if status != Some("complete") {
            continue;
        }
        let Some(message_hex) = msg.message.as_deref() else {
            continue;
        };
        if message_hex.is_empty() || message_hex == "0x" {
            continue;
        }
        if message_hex.len() > MAX_RAW_MESSAGE_BYTES * 2 + 2 {
            return Err(IrisError::Malformed("message hex too large".into()));
        }
        if let Some(att) = &msg.attestation {
            if att.len() > MAX_ATTESTATION_BYTES * 2 + 2 {
                return Err(IrisError::Malformed("attestation hex too large".into()));
            }
        }
        complete.push(msg);
    }

    match complete.len() {
        0 => Err(IrisError::Malformed("no complete message".into())),
        1 => Ok(complete[0].clone()),
        _ => Err(IrisError::Malformed("ambiguous messages".into())),
    }
}

#[async_trait]
impl IrisClient for ReqwestIrisClient {
    async fn fetch_burn_fees(
        &self,
        source_domain: u32,
        dest_domain: u32,
    ) -> Result<IrisFeeQuote, IrisError> {
        let url = format!(
            "{}/v2/burn/USDC/fees/{}/{}",
            self.base_url, source_domain, dest_domain
        );
        let resp = self.get_with_retries(&url).await?;
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(IrisError::Http("rate limited".into()));
        }
        if !resp.status().is_success() {
            return Err(IrisError::Http(format!("status {}", resp.status())));
        }
        let tiers: Vec<FeeTierResponse> = self.read_bounded_json(resp).await?;
        parse_burn_fee_tiers(tiers)
    }

    async fn poll_messages_by_tx(
        &self,
        source_domain: u32,
        tx_hash: &str,
    ) -> Result<IrisPollOutcome, IrisError> {
        // Iris hex lookups are domain-specific (EVM wants 0x…, Stellar rejects it).
        let query_hash = iris_query_tx_hash(source_domain, tx_hash);
        let url = format!(
            "{}/v2/messages/{}?transactionHash={}",
            self.base_url,
            source_domain,
            urlencoding::encode(&query_hash)
        );
        let resp = self.get_with_retries(&url).await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(IrisPollOutcome::NotFound);
        }
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(300);
            return Ok(IrisPollOutcome::RateLimited {
                retry_after_secs: retry.max(60),
            });
        }
        if !resp.status().is_success() {
            return Err(IrisError::Http(format!("status {}", resp.status())));
        }

        let body: MessagesResponse = self.read_bounded_json(resp).await?;

        if body.messages.is_empty() {
            return Ok(IrisPollOutcome::Pending);
        }

        let selected = match select_complete_v2_message(
            &body.messages,
            &query_hash,
            body.source_tx_hash.as_deref(),
        ) {
            Ok(msg) => msg,
            Err(IrisError::Malformed(reason)) if reason.contains("no complete") => {
                return Ok(IrisPollOutcome::Pending);
            }
            Err(e) => return Err(e),
        };

        let message_hex = selected
            .message
            .filter(|m| !m.is_empty() && m != "0x")
            .ok_or_else(|| IrisError::Malformed("complete message missing body".into()))?;

        let attestation = selected
            .attestation
            .filter(|a| !a.is_empty() && !a.eq_ignore_ascii_case("PENDING"));

        Ok(IrisPollOutcome::Complete(IrisMessage {
            message_hex,
            attestation_hex: attestation,
            cctp_version: 2,
            status: IrisMessageStatus::Complete,
            event_nonce: selected.event_nonce.unwrap_or_default(),
            // Prefer Iris echo; fall back to the hash we queried so validation
            // does not hard-fail when sandbox omits top-level sourceTxHash.
            source_tx_hash: body.source_tx_hash.or(Some(query_hash)),
        }))
    }

    async fn reattest(&self, nonce: &str) -> Result<(), IrisError> {
        check_str_len("nonce", nonce, 128).map_err(IrisError::Malformed)?;
        let url = format!(
            "{}/v2/reattest/{}",
            self.base_url,
            urlencoding::encode(nonce)
        );
        self.ensure_host(&url)?;
        let resp = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| IrisError::Http(redact_url(&e.to_string())))?;
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(IrisError::Http("rate limited".into()));
        }
        if !resp.status().is_success() {
            return Err(IrisError::Http(format!("status {}", resp.status())));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::config::CctpConfig;
    use wiremock::matchers::{method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fetch_burn_fees_parses_circle_tier_array() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/burn/USDC/fees/27/0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"finalityThreshold": 1000, "minimumFee": 1},
                {"finalityThreshold": 2000, "minimumFee": 0}
            ])))
            .mount(&server)
            .await;

        let cfg = CctpConfig {
            iris_base_url: server.uri(),
            ..CctpConfig::default_testnet()
        };
        let client = ReqwestIrisClient::from_config(&cfg).unwrap();
        let fees = client.fetch_burn_fees(27, 0).await.unwrap();
        assert_eq!(fees.standard_fee, "0");
        assert_eq!(fees.fast_fee.as_deref(), Some("1"));
    }

    #[tokio::test]
    async fn fetch_burn_fees_rejects_missing_standard_tier() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/burn/USDC/fees/27/0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"finalityThreshold": 1000, "minimumFee": 1}
            ])))
            .mount(&server)
            .await;
        let cfg = CctpConfig {
            iris_base_url: server.uri(),
            ..CctpConfig::default_testnet()
        };
        let client = ReqwestIrisClient::from_config(&cfg).unwrap();
        let err = client.fetch_burn_fees(27, 0).await.unwrap_err();
        assert!(matches!(err, IrisError::Malformed(_)));
    }

    #[tokio::test]
    async fn fetch_burn_fees_rejects_duplicate_standard_tier() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"finalityThreshold": 2000, "minimumFee": 0},
                {"finalityThreshold": 2000, "minimumFee": 1}
            ])))
            .mount(&server)
            .await;
        let cfg = CctpConfig {
            iris_base_url: server.uri(),
            ..CctpConfig::default_testnet()
        };
        let client = ReqwestIrisClient::from_config(&cfg).unwrap();
        let err = client.fetch_burn_fees(27, 0).await.unwrap_err();
        assert!(matches!(err, IrisError::Malformed(_)));
    }

    #[test]
    fn parse_live_shaped_fee_tiers() {
        let fees = parse_burn_fee_tiers(vec![
            FeeTierResponse {
                finality_threshold: 1000,
                minimum_fee: 0,
            },
            FeeTierResponse {
                finality_threshold: 2000,
                minimum_fee: 0,
            },
        ])
        .unwrap();
        assert_eq!(fees.standard_fee, "0");
        assert_eq!(fees.fast_fee.as_deref(), Some("0"));
    }

    #[tokio::test]
    async fn poll_pending_null_message_is_pending() {
        // Live Iris sandbox shape while awaiting finality (evm burns).
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"/v2/messages/0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "messages": [{
                    "attestation": "PENDING",
                    "message": null,
                    "eventNonce": "0x0754eb13210be9e7bae55a1448b2933d7c122aa79a533f6b35c3e47da91cf1a1",
                    "cctpVersion": 2,
                    "status": "pending_confirmations",
                    "decodedMessage": null,
                    "delayReason": null
                }],
                "sourceTxHash": "0x67fd76a2b5d463c119eb7a5fc26a0e4071f28e7cf612595bcf22241505c6bf5c"
            })))
            .mount(&server)
            .await;

        let cfg = CctpConfig {
            iris_base_url: server.uri(),
            ..CctpConfig::default_testnet()
        };
        let client = ReqwestIrisClient::from_config(&cfg).unwrap();
        let outcome = client
            .poll_messages_by_tx(
                0,
                "0x67fd76a2b5d463c119eb7a5fc26a0e4071f28e7cf612595bcf22241505c6bf5c",
            )
            .await
            .unwrap();
        assert_eq!(outcome, IrisPollOutcome::Pending);
    }

    #[tokio::test]
    async fn poll_pending_then_complete() {
        let server = MockServer::start().await;
        let base = server.uri();

        Mock::given(method("GET"))
            .and(path_regex(r"/v2/messages/27"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "messages": [{
                    "message": "0x",
                    "status": "pending_confirmations",
                    "cctpVersion": 2,
                    "eventNonce": "1"
                }],
                "sourceTxHash": "0xabc"
            })))
            .mount(&server)
            .await;

        let cfg = CctpConfig {
            iris_base_url: base.clone(),
            ..CctpConfig::default_testnet()
        };
        let client = ReqwestIrisClient::from_config(&cfg).unwrap();
        let outcome = client.poll_messages_by_tx(27, "0xabc").await.unwrap();
        assert_eq!(outcome, IrisPollOutcome::Pending);

        server.reset().await;
        Mock::given(method("GET"))
            .and(path_regex(r"/v2/messages/27"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "messages": [{
                    "message": "0x0001",
                    "attestation": "0xdead",
                    "status": "complete",
                    "cctpVersion": 2,
                    "eventNonce": "42"
                }],
                "sourceTxHash": "0xabc"
            })))
            .mount(&server)
            .await;

        let outcome = client.poll_messages_by_tx(27, "0xabc").await.unwrap();
        assert!(matches!(outcome, IrisPollOutcome::Complete(_)));
    }

    #[tokio::test]
    async fn poll_429_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "120"))
            .mount(&server)
            .await;

        let cfg = CctpConfig {
            iris_base_url: server.uri(),
            ..CctpConfig::default_testnet()
        };
        let client = ReqwestIrisClient::from_config(&cfg).unwrap();
        let outcome = client.poll_messages_by_tx(0, "0xabc").await.unwrap();
        assert!(matches!(
            outcome,
            IrisPollOutcome::RateLimited {
                retry_after_secs: 120
            }
        ));
    }

    #[tokio::test]
    async fn poll_wrong_cctp_version_rejected() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "messages": [{
                    "message": "0x01",
                    "status": "complete",
                    "cctpVersion": 1
                }]
            })))
            .mount(&server)
            .await;

        let cfg = CctpConfig {
            iris_base_url: server.uri(),
            ..CctpConfig::default_testnet()
        };
        let client = ReqwestIrisClient::from_config(&cfg).unwrap();
        let outcome = client.poll_messages_by_tx(0, "0xabc").await.unwrap();
        assert_eq!(outcome, IrisPollOutcome::Pending);
    }

    #[test]
    fn select_rejects_ambiguous_messages() {
        let messages = vec![
            MessageV2 {
                message: Some("0x01".into()),
                attestation: Some("0x02".into()),
                cctp_version: Some(2),
                status: Some("complete".into()),
                event_nonce: Some("1".into()),
            },
            MessageV2 {
                message: Some("0x03".into()),
                attestation: Some("0x04".into()),
                cctp_version: Some(2),
                status: Some("complete".into()),
                event_nonce: Some("2".into()),
            },
        ];
        let err = select_complete_v2_message(&messages, "0xabc", Some("0xabc")).unwrap_err();
        assert!(matches!(err, IrisError::Malformed(_)));
    }

    #[test]
    fn select_rejects_response_tx_hash_mismatch() {
        let messages = vec![MessageV2 {
            message: Some("0x01".into()),
            attestation: Some("0x02".into()),
            cctp_version: Some(2),
            status: Some("complete".into()),
            event_nonce: Some("1".into()),
        }];
        let err = select_complete_v2_message(&messages, "0xabc", Some("0xdef")).unwrap_err();
        assert!(matches!(err, IrisError::Malformed(_)));
    }

    #[test]
    fn iris_query_tx_hash_is_domain_specific() {
        let bare = "306b69605885205338acdcfa72c16fb1e34335033355c275965358842783bfcf";
        assert_eq!(iris_query_tx_hash(STELLAR_TESTNET_DOMAIN, bare), bare);
        assert_eq!(
            iris_query_tx_hash(STELLAR_TESTNET_DOMAIN, &format!("0x{bare}")),
            bare
        );
        assert_eq!(
            iris_query_tx_hash(0, &format!("0x{bare}")),
            format!("0x{bare}")
        );
        assert_eq!(iris_query_tx_hash(0, bare), format!("0x{bare}"));
    }

    #[test]
    fn ensure_host_rejects_evil_subdomain_in_client() {
        let cfg = CctpConfig::default_testnet();
        let client = ReqwestIrisClient::from_config(&cfg).unwrap();
        let err = client
            .ensure_host("https://iris-api-sandbox.circle.com.evil.com/v2/messages/0")
            .unwrap_err();
        assert!(matches!(err, IrisError::HostNotAllowlisted));
    }
}
