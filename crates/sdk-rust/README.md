# StellarRoute Rust SDK

The Rust SDK crate is `stellarroute-sdk`.

- Full Rust integration guide: [docs/sdk-rust/README.md](../docs/sdk-rust/README.md)
- CLI reference is preserved below.

## StellarRoute Rust SDK CLI

The CLI binary is available as:

```bash
cargo run -p stellarroute-sdk --bin stellarroute -- <command>
```

## Output formats

Use the global `--output` flag:

- `--output human` (default)
- `--output table`
- `--output json`

Invalid output values are rejected with an explicit validation error that lists accepted values.

## Exit codes

The CLI uses stable exit codes for scripting:

- `0`: success
- `2`: CLI usage/validation error (including invalid `--output`)
- `3`: invalid client configuration (for example, malformed `--api-url`)
- `4`: runtime/API error (HTTP/API/serialization failure)
