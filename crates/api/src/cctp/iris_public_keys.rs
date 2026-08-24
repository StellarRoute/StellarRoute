//! Iris `/v2/publicKeys` client with bounded cache and single-flight refresh.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use async_trait::async_trait;
use serde::Deserialize;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::cctp::attestation_crypto::eth_address_from_pubkey_xy;
use crate::cctp::bounds::{MAX_IRIS_JSON_BYTES, MAX_IRIS_PUBLIC_KEYS};
use crate::cctp::config::{parse_service_url, redact_url, CctpConfig, IRIS_SANDBOX_HOST};
use crate::metrics;

const UNCOMPRESSED_PUBKEY_LEN: usize = 65;
const XY_PUBKEY_LEN: usize = 64;

#[derive(Clone)]
pub struct IrisPublicKeySnapshot {
    pub addresses: Vec<[u8; 20]>,
    pub set_hash: [u8; 32],
    pub fetched_at: Instant,
}

impl fmt::Debug for IrisPublicKeySnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IrisPublicKeySnapshot")
            .field("count", &self.addresses.len())
            .field("set_hash", &hex::encode(self.set_hash))
            .finish()
    }
}

#[derive(Debug, Error)]
pub enum IrisPublicKeyError {
    #[error("not ready")]
    NotReady,
    #[error("http: {0}")]
    Http(String),
    #[error("malformed: {0}")]
    Malformed(String),
    #[error("stale")]
    Stale,
    #[error("rate limited")]
    RateLimited,
}

#[async_trait]
pub trait IrisPublicKeySource: Send + Sync {
    async fn fetch_public_keys(&self) -> Result<Vec<[u8; 20]>, IrisPublicKeyError>;
}

pub struct IrisPublicKeyCache {
    snapshot: ArcSwap<Option<IrisPublicKeySnapshot>>,
    ttl: Duration,
    stale_max: Duration,
    refresh_lock: Mutex<()>,
}

impl IrisPublicKeyCache {
    pub fn new(ttl: Duration, stale_max: Duration) -> Self {
        Self {
            snapshot: ArcSwap::from_pointee(None),
            ttl,
            stale_max,
            refresh_lock: Mutex::new(()),
        }
    }

    pub fn from_config(config: &CctpConfig) -> Self {
        Self::new(
            Duration::from_secs(config.iris_keys_ttl_secs),
            Duration::from_secs(config.iris_keys_stale_max_secs),
        )
    }

    pub fn snapshot(&self) -> Option<IrisPublicKeySnapshot> {
        self.snapshot.load_full().as_ref().clone()
    }

    pub fn is_healthy(&self) -> bool {
        self.snapshot()
            .map(|s| s.fetched_at.elapsed() <= self.ttl && s.fetched_at.elapsed() <= self.stale_max)
            .unwrap_or(false)
    }

    pub fn is_stale_beyond_max(&self) -> bool {
        self.snapshot()
            .map(|s| s.fetched_at.elapsed() > self.stale_max)
            .unwrap_or(true)
    }

    pub fn store(&self, addresses: Vec<[u8; 20]>) {
        let mut sorted = addresses;
        sorted.sort();
        sorted.dedup();
        let set_hash = crate::cctp::attester_set::iris_candidate_hash(&sorted);
        self.snapshot.store(Arc::new(Some(IrisPublicKeySnapshot {
            addresses: sorted,
            set_hash,
            fetched_at: Instant::now(),
        })));
    }

    pub async fn refresh<S: IrisPublicKeySource + ?Sized>(
        &self,
        source: &S,
    ) -> Result<(), IrisPublicKeyError> {
        let _guard = self.refresh_lock.lock().await;
        let addresses = source.fetch_public_keys().await?;
        if addresses.is_empty() {
            metrics::record_cctp_iris_keys_refresh("failure", "empty");
            return Err(IrisPublicKeyError::Malformed("empty key set".into()));
        }
        self.store(addresses);
        metrics::record_cctp_iris_keys_refresh("success", "ok");
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct PublicKeysResponse {
    #[serde(default, rename = "publicKeys")]
    public_keys: Vec<PublicKeyEntry>,
}

#[derive(Debug, Deserialize)]
struct PublicKeyEntry {
    #[serde(rename = "publicKey")]
    public_key: String,
    #[serde(rename = "cctpVersion", default)]
    cctp_version: Option<u32>,
}

pub struct ReqwestIrisPublicKeySource {
    client: reqwest::Client,
    base_url: String,
    allowed_host: String,
    max_retries: u32,
}

impl ReqwestIrisPublicKeySource {
    pub fn from_config(config: &CctpConfig) -> Result<Self, IrisPublicKeyError> {
        let parsed = parse_service_url(&config.iris_base_url)
            .map_err(|e| IrisPublicKeyError::Malformed(e.to_string()))?;
        if cfg!(test) && (parsed.host == "127.0.0.1" || parsed.host == "localhost") {
            // test mock server
        } else if parsed.host != IRIS_SANDBOX_HOST {
            return Err(IrisPublicKeyError::Malformed("iris host".into()));
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(config.iris_timeout_secs))
            .build()
            .map_err(|e| IrisPublicKeyError::Http(e.to_string()))?;
        Ok(Self {
            client,
            base_url: config.iris_base_url.trim_end_matches('/').to_string(),
            allowed_host: if cfg!(test)
                && (parsed.host == "127.0.0.1" || parsed.host == "localhost")
            {
                parsed.host
            } else {
                IRIS_SANDBOX_HOST.to_string()
            },
            max_retries: config.iris_max_retries,
        })
    }

    fn ensure_host(&self, url: &str) -> Result<(), IrisPublicKeyError> {
        let parsed =
            parse_service_url(url).map_err(|_| IrisPublicKeyError::Malformed("url".into()))?;
        if cfg!(test) && (parsed.host == "127.0.0.1" || parsed.host == "localhost") {
            return Ok(());
        }
        if parsed.host != self.allowed_host || parsed.scheme != "https" {
            return Err(IrisPublicKeyError::Malformed("host".into()));
        }
        Ok(())
    }

    async fn get_json(&self, url: &str) -> Result<PublicKeysResponse, IrisPublicKeyError> {
        self.ensure_host(url)?;
        let mut attempt = 0u32;
        loop {
            let resp = self.client.get(url).send().await;
            match resp {
                Ok(r) if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    return Err(IrisPublicKeyError::RateLimited);
                }
                Ok(r) if !r.status().is_success() => {
                    return Err(IrisPublicKeyError::Http(format!("status {}", r.status())));
                }
                Ok(r) => {
                    let bytes = r
                        .bytes()
                        .await
                        .map_err(|e| IrisPublicKeyError::Http(redact_url(&e.to_string())))?;
                    if bytes.len() > MAX_IRIS_JSON_BYTES {
                        return Err(IrisPublicKeyError::Malformed("too large".into()));
                    }
                    return serde_json::from_slice(&bytes)
                        .map_err(|e| IrisPublicKeyError::Malformed(e.to_string()));
                }
                Err(_e) if attempt < self.max_retries => {
                    attempt += 1;
                    tokio::time::sleep(Duration::from_millis(50 + attempt as u64 * 25)).await;
                }
                Err(e) => return Err(IrisPublicKeyError::Http(redact_url(&e.to_string()))),
            }
        }
    }
}

#[async_trait]
impl IrisPublicKeySource for ReqwestIrisPublicKeySource {
    async fn fetch_public_keys(&self) -> Result<Vec<[u8; 20]>, IrisPublicKeyError> {
        let url = format!("{}/v2/publicKeys", self.base_url);
        let body = self.get_json(&url).await?;
        parse_iris_public_keys(&body)
    }
}

pub(crate) fn parse_iris_public_keys(
    body: &PublicKeysResponse,
) -> Result<Vec<[u8; 20]>, IrisPublicKeyError> {
    if body.public_keys.is_empty() {
        return Err(IrisPublicKeyError::Malformed("empty".into()));
    }
    let mut out = Vec::new();
    for entry in &body.public_keys {
        let version = entry.cctp_version.unwrap_or(0);
        if version != 2 {
            continue;
        }
        let addr = parse_uncompressed_pubkey_hex(&entry.public_key)?;
        out.push(addr);
    }
    if out.is_empty() {
        return Err(IrisPublicKeyError::Malformed("no v2 keys".into()));
    }
    if out.len() > MAX_IRIS_PUBLIC_KEYS {
        return Err(IrisPublicKeyError::Malformed("too many keys".into()));
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn parse_uncompressed_pubkey_hex(hex_key: &str) -> Result<[u8; 20], IrisPublicKeyError> {
    let trimmed = hex_key.trim();
    let hex = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    if hex.len() != UNCOMPRESSED_PUBKEY_LEN * 2 || !hex.starts_with("04") {
        return Err(IrisPublicKeyError::Malformed("pubkey format".into()));
    }
    let bytes = hex::decode(hex).map_err(|_| IrisPublicKeyError::Malformed("pubkey hex".into()))?;
    if bytes.len() != UNCOMPRESSED_PUBKEY_LEN || bytes[0] != 0x04 {
        return Err(IrisPublicKeyError::Malformed("pubkey prefix".into()));
    }
    let mut xy = [0u8; XY_PUBKEY_LEN];
    xy.copy_from_slice(&bytes[1..]);
    Ok(eth_address_from_pubkey_xy(&xy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn parse_v2_key_response() {
        let pk = format!("0x04{}", "22".repeat(64));
        let body = PublicKeysResponse {
            public_keys: vec![PublicKeyEntry {
                public_key: pk,
                cctp_version: Some(2),
            }],
        };
        let addrs = parse_iris_public_keys(&body).expect("parse");
        assert_eq!(addrs.len(), 1);
    }

    #[test]
    fn rejects_wrong_version() {
        let body = PublicKeysResponse {
            public_keys: vec![PublicKeyEntry {
                public_key: "0x04".to_string() + &"00".repeat(64),
                cctp_version: Some(1),
            }],
        };
        assert!(matches!(
            parse_iris_public_keys(&body),
            Err(IrisPublicKeyError::Malformed(_))
        ));
    }

    #[tokio::test]
    async fn fetch_from_mock_server() {
        let server = MockServer::start().await;
        // Valid uncompressed secp256k1 point encoding (format check only).
        let pk = format!("0x04{}", "11".repeat(64));
        Mock::given(method("GET"))
            .and(path("/v2/publicKeys"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "publicKeys": [{
                    "publicKey": pk,
                    "cctpVersion": 2
                }]
            })))
            .mount(&server)
            .await;

        let mut cfg = CctpConfig::default_testnet();
        cfg.iris_base_url = server.uri();
        let source = ReqwestIrisPublicKeySource::from_config(&cfg).unwrap();
        let result = source.fetch_public_keys().await;
        assert!(result.is_ok(), "fetch failed: {:?}", result.err());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn cache_single_flight_refresh() {
        let cache = IrisPublicKeyCache::new(Duration::from_secs(900), Duration::from_secs(86400));
        let addresses = vec![[1u8; 20], [2u8; 20]];
        cache.store(addresses.clone());
        let snap = cache.snapshot().unwrap();
        assert_eq!(snap.addresses, addresses);
        assert!(cache.is_healthy());
    }
}
