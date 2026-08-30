//! Transfer access capability tokens — hash-only persistence; HMAC recovery for idempotent quotes.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::cctp::bounds::check_str_len;

pub const TRANSFER_ACCESS_HEADER: &str = "x-cctp-transfer-access";
pub const ACCESS_TOKEN_HMAC_ENV: &str = "CCTP_ACCESS_TOKEN_HMAC_KEY";
pub const ACCESS_TOKEN_HMAC_PREVIOUS_ENV: &str = "CCTP_ACCESS_TOKEN_HMAC_PREVIOUS_KEYS";
pub const ACCESS_TOKEN_BYTES: usize = 32;
pub const MIN_HMAC_KEY_BYTES: usize = 32;
pub const MAX_PREVIOUS_HMAC_KEYS: usize = 2;
pub const MAX_ACCESS_TOKEN_LEN: usize = 128;

type HmacSha256 = Hmac<Sha256>;

/// Current + bounded previous HMAC keys for idempotent quote token derivation and rotation replay.
#[derive(Clone)]
pub struct CctpAccessTokenKeyRing {
    current: Vec<u8>,
    previous: Vec<Vec<u8>>,
}

impl std::fmt::Debug for CctpAccessTokenKeyRing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CctpAccessTokenKeyRing([REDACTED])")
    }
}

impl CctpAccessTokenKeyRing {
    pub fn from_env_when_enabled(enabled: bool) -> Result<Option<Self>, String> {
        let raw = match std::env::var(ACCESS_TOKEN_HMAC_ENV) {
            Ok(v) if !v.trim().is_empty() => v,
            _ if enabled => {
                return Err(format!(
                    "{ACCESS_TOKEN_HMAC_ENV} is required when CCTP_ENABLED=true (>= {MIN_HMAC_KEY_BYTES} random bytes, base64 or hex)"
                ));
            }
            _ => return Ok(None),
        };
        let current = parse_secret_bytes(&raw)?;
        if current.len() < MIN_HMAC_KEY_BYTES {
            return Err(format!(
                "{ACCESS_TOKEN_HMAC_ENV} must decode to at least {MIN_HMAC_KEY_BYTES} bytes"
            ));
        }
        let previous = parse_previous_keys_from_env()?;
        Ok(Some(Self { current, previous }))
    }

    pub fn from_single_key(bytes: Vec<u8>) -> Self {
        assert!(bytes.len() >= MIN_HMAC_KEY_BYTES);
        Self {
            current: bytes,
            previous: Vec::new(),
        }
    }

    pub fn with_previous(mut self, previous: Vec<Vec<u8>>) -> Self {
        self.previous = previous.into_iter().take(MAX_PREVIOUS_HMAC_KEYS).collect();
        self
    }

    /// New idempotent quotes always derive with the current key.
    pub fn derive_idempotent_token(
        &self,
        idempotency_key: &str,
        canonical_request_hash: &str,
        transfer_id: Uuid,
    ) -> String {
        derive_idempotent_token_with_key(
            &self.current,
            idempotency_key,
            canonical_request_hash,
            transfer_id,
        )
    }

    /// Replay after rotation: try current then previous keys until one matches the stored hash.
    pub fn recover_idempotent_token(
        &self,
        idempotency_key: &str,
        canonical_request_hash: &str,
        transfer_id: Uuid,
        stored_hash: &str,
    ) -> Option<String> {
        for key in std::iter::once(&self.current).chain(self.previous.iter()) {
            let token = derive_idempotent_token_with_key(
                key,
                idempotency_key,
                canonical_request_hash,
                transfer_id,
            );
            if access_tokens_match(stored_hash, &token) {
                return Some(token);
            }
        }
        None
    }
}

fn parse_previous_keys_from_env() -> Result<Vec<Vec<u8>>, String> {
    let Ok(raw) = std::env::var(ACCESS_TOKEN_HMAC_PREVIOUS_ENV) else {
        return Ok(Vec::new());
    };
    let mut keys = Vec::new();
    for part in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if keys.len() >= MAX_PREVIOUS_HMAC_KEYS {
            return Err(format!(
                "{ACCESS_TOKEN_HMAC_PREVIOUS_ENV} accepts at most {MAX_PREVIOUS_HMAC_KEYS} keys"
            ));
        }
        let bytes = parse_secret_bytes(part)?;
        if bytes.len() < MIN_HMAC_KEY_BYTES {
            return Err(format!(
                "each {ACCESS_TOKEN_HMAC_PREVIOUS_ENV} entry must decode to at least {MIN_HMAC_KEY_BYTES} bytes"
            ));
        }
        keys.push(bytes);
    }
    Ok(keys)
}

fn derive_idempotent_token_with_key(
    key: &[u8],
    idempotency_key: &str,
    canonical_request_hash: &str,
    transfer_id: Uuid,
) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length validated at load");
    mac.update(b"cctp-transfer-access-v1\0");
    mac.update(idempotency_key.as_bytes());
    mac.update(b"\0");
    mac.update(canonical_request_hash.as_bytes());
    mac.update(b"\0");
    mac.update(transfer_id.as_bytes());
    let digest = mac.finalize().into_bytes();
    URL_SAFE_NO_PAD.encode(digest)
}

fn parse_secret_bytes(raw: &str) -> Result<Vec<u8>, String> {
    let trimmed = raw.trim();
    if let Ok(bytes) = hex::decode(trimmed) {
        return Ok(bytes);
    }
    if let Ok(bytes) = URL_SAFE_NO_PAD.decode(trimmed) {
        return Ok(bytes);
    }
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(trimmed) {
        return Ok(bytes);
    }
    Err(format!(
        "{ACCESS_TOKEN_HMAC_ENV} must be hex, standard base64, or base64url"
    ))
}

/// One-time CSPRNG token for non-idempotent quotes (returned once; only hash stored).
pub fn generate_ephemeral_access_token() -> (String, String) {
    let mut raw = [0u8; ACCESS_TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut raw);
    let token = URL_SAFE_NO_PAD.encode(raw);
    let hash = hash_access_token(&token);
    (token, hash)
}

pub fn hash_access_token(token: &str) -> String {
    let digest = Sha256::digest(token.trim().as_bytes());
    hex::encode(digest)
}

pub fn hash_lease_owner(owner_nonce: &str) -> String {
    hash_access_token(owner_nonce)
}

pub fn validate_access_token_format(token: &str) -> Result<(), String> {
    let trimmed = token.trim();
    check_str_len("access_token", trimmed, MAX_ACCESS_TOKEN_LEN)?;
    if trimmed.is_empty() {
        return Err("access token required".into());
    }
    Ok(())
}

pub fn access_tokens_match(persisted_hash: &str, presented: &str) -> bool {
    validate_access_token_format(presented).is_ok()
        && constant_time_eq(
            persisted_hash.as_bytes(),
            hash_access_token(presented).as_bytes(),
        )
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Legacy helper for tests that predate HMAC idempotency.
pub fn generate_access_token() -> (String, String) {
    generate_ephemeral_access_token()
}

pub fn test_access_token_keyring() -> CctpAccessTokenKeyRing {
    CctpAccessTokenKeyRing::from_single_key(vec![0x42u8; MIN_HMAC_KEY_BYTES])
}

pub fn test_access_token_hash() -> String {
    hash_access_token("test-token-for-unit-tests-only")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_token_roundtrip_hash() {
        let (token, hash) = generate_ephemeral_access_token();
        assert!(access_tokens_match(&hash, &token));
        assert!(!access_tokens_match(&hash, "wrong"));
    }

    #[test]
    fn idempotent_token_is_deterministic() {
        let ring = test_access_token_keyring();
        let id = Uuid::new_v4();
        let a = ring.derive_idempotent_token("idem-1", "abc123", id);
        let b = ring.derive_idempotent_token("idem-1", "abc123", id);
        assert_eq!(a, b);
        assert_ne!(ring.derive_idempotent_token("idem-2", "abc123", id), a);
    }

    #[test]
    fn rotation_replay_recovers_with_previous_key() {
        let old_key = vec![0x11u8; MIN_HMAC_KEY_BYTES];
        let new_key = vec![0x22u8; MIN_HMAC_KEY_BYTES];
        let ring = CctpAccessTokenKeyRing::from_single_key(new_key.clone())
            .with_previous(vec![old_key.clone()]);
        let id = Uuid::new_v4();
        let token_old = derive_idempotent_token_with_key(&old_key, "k", "hash", id);
        let hash = hash_access_token(&token_old);
        let recovered = ring
            .recover_idempotent_token("k", "hash", id, &hash)
            .unwrap();
        assert_eq!(recovered, token_old);
        assert_ne!(ring.derive_idempotent_token("k", "hash", id), token_old);
    }

    #[test]
    fn hmac_key_parsing_hex_and_base64() {
        let hex_key = hex::encode([1u8; 32]);
        let parsed = parse_secret_bytes(&hex_key).unwrap();
        assert_eq!(parsed.len(), 32);
    }
}
