use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use k256::ecdsa::SigningKey;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use tiny_keccak::{Hasher, Keccak};

const SEPOLIA_CHAIN_ID: u64 = 11_155_111;
const SEPOLIA_MESSAGE_TRANSMITTER: &str =
    stellarroute_api::cctp::config::SEPOLIA_MESSAGE_TRANSMITTER;
const RECEIVE_MESSAGE_SELECTOR: [u8; 4] = [0x57, 0xec, 0xfd, 0x28];
const MAX_CALLDATA_BYTES: usize = 256 * 1024;

#[derive(Debug, Parser)]
#[command(about = "Local CCTP EVM signer; secret keys are read only from secure files")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Address {
        #[arg(long)]
        key_file: PathBuf,
    },
    Send {
        #[arg(long)]
        key_file: PathBuf,
        #[arg(long)]
        request_file: PathBuf,
        #[arg(long)]
        rpc_file: PathBuf,
    },
    DryRun {
        #[arg(long)]
        key_file: PathBuf,
        #[arg(long)]
        request_file: PathBuf,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignRequest {
    chain_id: u64,
    recipient: String,
    contract: String,
    to: String,
    data: String,
    value: String,
    max_gas_limit: u64,
    #[serde(default)]
    fixture: Option<TxFields>,
    #[serde(default)]
    expected_tx_hash: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TxFields {
    nonce: u64,
    gas_limit: u64,
    max_priority_fee_per_gas: u128,
    max_fee_per_gas: u128,
}

struct ValidatedRequest {
    recipient: [u8; 20],
    to: [u8; 20],
    data: Vec<u8>,
    value: u128,
    max_gas_limit: u64,
}

struct RpcClient {
    url: String,
    http: reqwest::Client,
}

impl RpcClient {
    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        let response = self
            .http
            .post(&self.url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            }))
            .send()
            .await
            .context("EVM RPC request failed")?;
        if !response.status().is_success() {
            bail!("EVM RPC returned HTTP {}", response.status());
        }
        let envelope: Value = response
            .json()
            .await
            .context("EVM RPC response was not valid JSON")?;
        if envelope.get("error").is_some_and(|error| !error.is_null()) {
            bail!("EVM RPC returned an error");
        }
        serde_json::from_value(
            envelope
                .get("result")
                .cloned()
                .ok_or_else(|| anyhow!("EVM RPC response omitted result"))?,
        )
        .context("EVM RPC result had an unexpected shape")
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("cctp-evm-signer: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Address { key_file } => {
            let signing_key = read_secure_key(&key_file)?;
            println!("{}", format_address(&signer_address(&signing_key)));
        }
        Command::DryRun {
            key_file,
            request_file,
        } => {
            let signing_key = read_secure_key(&key_file)?;
            let request = read_request(&request_file)?;
            let validated = validate_request(&request, signer_address(&signing_key))?;
            let fields = request
                .fixture
                .as_ref()
                .ok_or_else(|| anyhow!("dry-run request is missing fixture transaction fields"))?;
            validate_fields(fields, validated.max_gas_limit)?;
            let (_, hash) = sign_transaction(&signing_key, &validated, fields)?;
            let hash = format_hash(&hash);
            if let Some(expected) = request.expected_tx_hash.as_deref() {
                if !expected.eq_ignore_ascii_case(&hash) {
                    bail!("deterministic transaction hash did not match fixture");
                }
            }
            println!("{hash}");
        }
        Command::Send {
            key_file,
            request_file,
            rpc_file,
        } => {
            let signing_key = read_secure_key(&key_file)?;
            let request = read_request(&request_file)?;
            if request.fixture.is_some() || request.expected_tx_hash.is_some() {
                bail!("live signing request contains dry-run-only fields");
            }
            let validated = validate_request(&request, signer_address(&signing_key))?;
            let rpc = read_rpc_client(&rpc_file)?;
            let chain_hex: String = rpc.call("eth_chainId", json!([])).await?;
            if parse_hex_u128(&chain_hex)? != u128::from(SEPOLIA_CHAIN_ID) {
                bail!("RPC chain ID is not Sepolia");
            }
            let fields = resolve_fields(&rpc, &validated).await?;
            validate_fields(&fields, validated.max_gas_limit)?;
            let (raw, hash) = sign_transaction(&signing_key, &validated, &fields)?;
            let expected_hash = format_hash(&hash);
            let raw_hex = format!("0x{}", hex::encode(raw));
            let broadcast: Result<String> =
                rpc.call("eth_sendRawTransaction", json!([raw_hex])).await;
            match broadcast {
                Ok(returned_hash) if returned_hash.eq_ignore_ascii_case(&expected_hash) => {
                    println!("{expected_hash}");
                }
                Ok(_) => bail!("RPC returned a transaction hash that did not match local signing"),
                Err(broadcast_error) => {
                    let lookup: Result<Value> = rpc
                        .call("eth_getTransactionByHash", json!([expected_hash]))
                        .await;
                    if lookup.is_ok_and(|value| !value.is_null()) {
                        println!("{expected_hash}");
                    } else {
                        return Err(broadcast_error)
                            .context("broadcast failed and transaction hash lookup was empty");
                    }
                }
            }
        }
    }
    Ok(())
}

fn read_secure_key(path: &Path) -> Result<SigningKey> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("cannot inspect key file {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        bail!("key path must be a regular file, not a symlink");
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o777 != 0o600 {
        bail!("key file mode must be exactly 0600");
    }

    let mut secret = fs::read(path).context("cannot read key file")?;
    let start = secret
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .ok_or_else(|| anyhow!("key file is empty"))?;
    let end = secret
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .expect("non-empty key bytes")
        + 1;
    let encoded = std::str::from_utf8(&secret[start..end]).context("key file is not UTF-8")?;
    let encoded = encoded.strip_prefix("0x").unwrap_or(encoded);
    if encoded.len() != 64 || !encoded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        secret.fill(0);
        bail!("key file must contain exactly one 32-byte hex private key");
    }
    let decoded = hex::decode(encoded).context("key file contains invalid hex")?;
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&decoded);
    secret.fill(0);
    let signing_key = SigningKey::from_bytes((&key_bytes).into())
        .map_err(|_| anyhow!("key is not a valid secp256k1 scalar"));
    key_bytes.fill(0);
    signing_key
}

fn read_request(path: &Path) -> Result<SignRequest> {
    let metadata = fs::symlink_metadata(path).context("cannot inspect request file")?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        bail!("request path must be a regular file, not a symlink");
    }
    serde_json::from_slice(&fs::read(path).context("cannot read request file")?)
        .context("invalid signing request")
}

fn read_rpc_client(path: &Path) -> Result<RpcClient> {
    let metadata = fs::symlink_metadata(path).context("cannot inspect RPC URL file")?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        bail!("RPC URL path must be a regular file, not a symlink");
    }
    let url = fs::read_to_string(path)
        .context("cannot read RPC URL file")?
        .trim()
        .to_owned();
    let parsed = reqwest::Url::parse(&url).context("RPC URL is invalid")?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        bail!("RPC URL must be credential-free HTTPS");
    }
    Ok(RpcClient {
        url,
        http: reqwest::Client::new(),
    })
}

fn validate_request(request: &SignRequest, signer: [u8; 20]) -> Result<ValidatedRequest> {
    if request.chain_id != SEPOLIA_CHAIN_ID {
        bail!("signing request chain ID is not Sepolia");
    }
    let recipient = parse_address(&request.recipient)?;
    if recipient != signer {
        bail!("signing key does not match the expected recipient");
    }
    let contract = parse_address(&request.contract)?;
    let pinned_contract = parse_address(SEPOLIA_MESSAGE_TRANSMITTER)?;
    let to = parse_address(&request.to)?;
    if contract != pinned_contract || to != pinned_contract {
        bail!("mint destination is not the pinned Sepolia MessageTransmitter");
    }
    let value = parse_quantity(&request.value)?;
    if value != 0 {
        bail!("CCTP mint transaction value must be zero");
    }
    if request.max_gas_limit < 21_000 || request.max_gas_limit > 2_000_000 {
        bail!("gas safety cap is outside the permitted range");
    }
    let data = parse_hex_bytes(&request.data)?;
    if data.len() < 4 || data.len() > MAX_CALLDATA_BYTES {
        bail!("mint calldata length is invalid");
    }
    if data[..4] != RECEIVE_MESSAGE_SELECTOR {
        bail!("mint calldata is not receiveMessage(bytes,bytes)");
    }
    let mut padded_recipient = [0u8; 32];
    padded_recipient[12..].copy_from_slice(&recipient);
    if !data.windows(32).any(|window| window == padded_recipient) {
        bail!("mint calldata does not contain the expected recipient");
    }
    Ok(ValidatedRequest {
        recipient,
        to,
        data,
        value,
        max_gas_limit: request.max_gas_limit,
    })
}

async fn resolve_fields(rpc: &RpcClient, request: &ValidatedRequest) -> Result<TxFields> {
    let from = format_address(&request.recipient);
    let to = format_address(&request.to);
    let data = format!("0x{}", hex::encode(&request.data));
    let value = format!("0x{:x}", request.value);

    let nonce_hex: String = rpc
        .call("eth_getTransactionCount", json!([from, "pending"]))
        .await?;
    let estimate_hex: String = rpc
        .call(
            "eth_estimateGas",
            json!([{
                "from": from,
                "to": to,
                "data": data,
                "value": value,
            }]),
        )
        .await?;
    let gas_price_hex: String = rpc.call("eth_gasPrice", json!([])).await?;
    let block: Value = rpc
        .call("eth_getBlockByNumber", json!(["latest", false]))
        .await?;
    let base_fee_hex = block
        .get("baseFeePerGas")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("latest block omitted baseFeePerGas"))?;

    let nonce = u64::try_from(parse_hex_u128(&nonce_hex)?).context("nonce exceeds u64")?;
    let estimate =
        u64::try_from(parse_hex_u128(&estimate_hex)?).context("gas estimate exceeds u64")?;
    let gas_limit = estimate
        .checked_add(estimate / 5)
        .ok_or_else(|| anyhow!("gas estimate overflow"))?;
    let gas_price = parse_hex_u128(&gas_price_hex)?;
    let base_fee = parse_hex_u128(base_fee_hex)?;
    let priority = gas_price.saturating_sub(base_fee).max(1_000_000);
    let max_fee = base_fee
        .checked_mul(2)
        .and_then(|fee| fee.checked_add(priority))
        .ok_or_else(|| anyhow!("fee calculation overflow"))?;
    Ok(TxFields {
        nonce,
        gas_limit,
        max_priority_fee_per_gas: priority,
        max_fee_per_gas: max_fee,
    })
}

fn validate_fields(fields: &TxFields, max_gas_limit: u64) -> Result<()> {
    if fields.gas_limit < 21_000 || fields.gas_limit > max_gas_limit {
        bail!("resolved gas limit violates the signing request safety cap");
    }
    if fields.max_priority_fee_per_gas == 0
        || fields.max_fee_per_gas < fields.max_priority_fee_per_gas
    {
        bail!("resolved EIP-1559 fees are invalid");
    }
    Ok(())
}

fn sign_transaction(
    signing_key: &SigningKey,
    request: &ValidatedRequest,
    fields: &TxFields,
) -> Result<(Vec<u8>, [u8; 32])> {
    let unsigned = rlp_list(&[
        rlp_u128(u128::from(SEPOLIA_CHAIN_ID)),
        rlp_u128(u128::from(fields.nonce)),
        rlp_u128(fields.max_priority_fee_per_gas),
        rlp_u128(fields.max_fee_per_gas),
        rlp_u128(u128::from(fields.gas_limit)),
        rlp_bytes(&request.to),
        rlp_u128(request.value),
        rlp_bytes(&request.data),
        vec![0xc0],
    ]);
    let mut signing_payload = Vec::with_capacity(1 + unsigned.len());
    signing_payload.push(0x02);
    signing_payload.extend_from_slice(&unsigned);
    let digest = keccak256(&signing_payload);
    let (signature, recovery_id) = signing_key
        .sign_prehash_recoverable(&digest)
        .expect("validated secp256k1 key can sign a 32-byte digest");
    if recovery_id.to_byte() > 1 {
        bail!("signature recovery ID is not valid for an EIP-1559 transaction");
    }
    let signature = signature.to_bytes();
    let signed = rlp_list(&[
        rlp_u128(u128::from(SEPOLIA_CHAIN_ID)),
        rlp_u128(u128::from(fields.nonce)),
        rlp_u128(fields.max_priority_fee_per_gas),
        rlp_u128(fields.max_fee_per_gas),
        rlp_u128(u128::from(fields.gas_limit)),
        rlp_bytes(&request.to),
        rlp_u128(request.value),
        rlp_bytes(&request.data),
        vec![0xc0],
        rlp_u128(u128::from(recovery_id.to_byte())),
        rlp_bytes(trim_integer(&signature[..32])),
        rlp_bytes(trim_integer(&signature[32..])),
    ]);
    let mut raw = Vec::with_capacity(1 + signed.len());
    raw.push(0x02);
    raw.extend_from_slice(&signed);
    let hash = keccak256(&raw);
    Ok((raw, hash))
}

fn signer_address(signing_key: &SigningKey) -> [u8; 20] {
    let encoded = signing_key.verifying_key().to_encoded_point(false);
    let hash = keccak256(&encoded.as_bytes()[1..]);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..]);
    address
}

fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(input);
    hasher.finalize(&mut output);
    output
}

fn rlp_u128(value: u128) -> Vec<u8> {
    if value == 0 {
        return vec![0x80];
    }
    let bytes = value.to_be_bytes();
    rlp_bytes(trim_integer(&bytes))
}

fn trim_integer(bytes: &[u8]) -> &[u8] {
    let first_nonzero = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    &bytes[first_nonzero..]
}

fn rlp_bytes(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() == 1 && bytes[0] < 0x80 {
        return vec![bytes[0]];
    }
    let mut encoded = rlp_length(bytes.len(), 0x80, 0xb7);
    encoded.extend_from_slice(bytes);
    encoded
}

fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload_len = items.iter().map(Vec::len).sum();
    let mut encoded = rlp_length(payload_len, 0xc0, 0xf7);
    for item in items {
        encoded.extend_from_slice(item);
    }
    encoded
}

fn rlp_length(length: usize, short_offset: u8, long_offset: u8) -> Vec<u8> {
    if length <= 55 {
        return vec![short_offset + length as u8];
    }
    let bytes = length.to_be_bytes();
    let length_bytes = trim_integer(&bytes);
    let mut encoded = Vec::with_capacity(1 + length_bytes.len());
    encoded.push(long_offset + length_bytes.len() as u8);
    encoded.extend_from_slice(length_bytes);
    encoded
}

fn parse_address(value: &str) -> Result<[u8; 20]> {
    let decoded = parse_fixed_hex(value, 20).context("invalid EVM address")?;
    let mut address = [0u8; 20];
    address.copy_from_slice(&decoded);
    Ok(address)
}

fn parse_hex_bytes(value: &str) -> Result<Vec<u8>> {
    let encoded = value
        .strip_prefix("0x")
        .ok_or_else(|| anyhow!("hex calldata must start with 0x"))?;
    if encoded.len() % 2 != 0 || !encoded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("hex calldata is malformed");
    }
    hex::decode(encoded).context("hex calldata is malformed")
}

fn parse_fixed_hex(value: &str, bytes: usize) -> Result<Vec<u8>> {
    let encoded = value
        .strip_prefix("0x")
        .ok_or_else(|| anyhow!("hex value must start with 0x"))?;
    if encoded.len() != bytes * 2 || !encoded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("hex value has the wrong length or characters");
    }
    hex::decode(encoded).context("hex value is malformed")
}

fn parse_quantity(value: &str) -> Result<u128> {
    if value.starts_with("0x") {
        parse_hex_u128(value)
    } else {
        value.parse::<u128>().context("invalid decimal quantity")
    }
}

fn parse_hex_u128(value: &str) -> Result<u128> {
    let encoded = value
        .strip_prefix("0x")
        .ok_or_else(|| anyhow!("RPC quantity must start with 0x"))?;
    if encoded.is_empty() || !encoded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("RPC quantity is malformed");
    }
    u128::from_str_radix(encoded, 16).context("RPC quantity exceeds u128")
}

fn format_address(address: &[u8; 20]) -> String {
    format!("0x{}", hex::encode(address))
}

fn format_hash(hash: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(hash))
}
