use std::{
    ffi::OsStr,
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const MARKER_ADDRESS: &str = "0x7e5f4552091a69125d5dfcb7b8c2659029395bdf";
const EXPECTED_FIXTURE_HASH: &str =
    "0x7b47b66cef01df36b34bef6c1d438dc32e2f96b7a1c3c245e3c4d594596710cd";

fn marker_private_key() -> String {
    format!("0x{}1", "0".repeat(63))
}

fn signer_binary() -> &'static str {
    env!("CARGO_BIN_EXE_cctp-evm-signer")
}

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "stellarroute-cctp-signer-{name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&path).expect("create test directory");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("secure test directory");
    path
}

fn write_file(path: &Path, contents: &str, mode: u32) {
    fs::write(path, contents).expect("write fixture");
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("set fixture permissions");
}

fn fixture_request() -> String {
    format!(
        r#"{{
  "chain_id": 11155111,
  "recipient": "{MARKER_ADDRESS}",
  "contract": "0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275",
  "to": "0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275",
  "data": "0x57ecfd28000000000000000000000000{}",
  "value": "0",
  "max_gas_limit": 1000000,
  "fixture": {{
    "nonce": 7,
    "gas_limit": 150000,
    "max_priority_fee_per_gas": 1000000000,
    "max_fee_per_gas": 3000000000
  }},
  "expected_tx_hash": "{EXPECTED_FIXTURE_HASH}"
}}"#,
        &MARKER_ADDRESS[2..]
    )
}

#[test]
fn dry_run_keeps_marker_out_of_child_process_metadata() {
    let dir = temp_dir("argv");
    let key_path = dir.join("key");
    let request_path = dir.join("request.json");
    let marker_private_key = marker_private_key();
    write_file(&key_path, &marker_private_key, 0o600);
    write_file(&request_path, &fixture_request(), 0o600);

    let mut command = Command::new(signer_binary());
    command
        .env_clear()
        .args(["dry-run", "--key-file"])
        .arg(&key_path)
        .arg("--request-file")
        .arg(&request_path);

    for arg in command.get_args() {
        assert!(!arg.to_string_lossy().contains(&marker_private_key));
    }
    for (key, value) in command.get_envs() {
        assert!(!key.to_string_lossy().contains(&marker_private_key));
        assert!(!value
            .unwrap_or_else(|| OsStr::new(""))
            .to_string_lossy()
            .contains(&marker_private_key));
    }

    let output = command.output().expect("run dry signer fixture");
    assert!(
        output.status.success(),
        "signer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output
        .stdout
        .windows(marker_private_key.len())
        .any(|window| { window == marker_private_key.as_bytes() }));
    assert!(!output
        .stderr
        .windows(marker_private_key.len())
        .any(|window| { window == marker_private_key.as_bytes() }));
    assert_eq!(
        String::from_utf8(output.stdout)
            .expect("hash is UTF-8")
            .trim(),
        EXPECTED_FIXTURE_HASH
    );

    fs::remove_dir_all(dir).expect("remove test directory");
}

#[test]
fn rejects_insecure_or_symlinked_key_files_without_echoing_secret() {
    let dir = temp_dir("permissions");
    let open_key = dir.join("open-key");
    let marker_private_key = marker_private_key();
    write_file(&open_key, &marker_private_key, 0o644);

    let output = Command::new(signer_binary())
        .env_clear()
        .args(["address", "--key-file"])
        .arg(&open_key)
        .output()
        .expect("run permission rejection");
    assert!(!output.status.success());
    assert!(!output
        .stdout
        .windows(marker_private_key.len())
        .any(|window| { window == marker_private_key.as_bytes() }));
    assert!(!output
        .stderr
        .windows(marker_private_key.len())
        .any(|window| { window == marker_private_key.as_bytes() }));

    fs::set_permissions(&open_key, fs::Permissions::from_mode(0o600)).expect("secure source key");
    let key_link = dir.join("key-link");
    symlink(&open_key, &key_link).expect("create key symlink");
    let output = Command::new(signer_binary())
        .env_clear()
        .args(["address", "--key-file"])
        .arg(&key_link)
        .output()
        .expect("run symlink rejection");
    assert!(!output.status.success());

    fs::remove_dir_all(dir).expect("remove test directory");
}

#[test]
fn signed_live_shell_is_syntax_valid_and_redacted() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/cctp-signed-live-stellar-to-sepolia-proof.sh");
    let status = Command::new("bash")
        .args(["-n"])
        .arg(&script)
        .status()
        .expect("run bash syntax check");
    assert!(status.success());

    let source = fs::read_to_string(script).expect("read signed-live harness");
    for forbidden in [
        "--private-key",
        "SEPOLIA_PRIVATE_KEY",
        "cast wallet new",
        "cast send",
        "cat \"$EVM_KEY_FILE\"",
        "$(<\"$EVM_KEY_FILE\")",
    ] {
        assert!(
            !source.contains(forbidden),
            "signed-live harness contains forbidden secret handling: {forbidden}"
        );
    }
    assert!(source.contains("\"$EVM_SIGNER_BIN\" send"));
    assert!(source.contains("--key-file \"$EVM_KEY_FILE\""));
}
