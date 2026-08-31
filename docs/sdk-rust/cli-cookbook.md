# StellarRoute CLI cookbook

A copy-paste cookbook for the four existing `stellarroute` CLI subcommands:
`health`, `pairs`, `quote`, and `orderbook`. This documents the current CLI
behavior only — it does **not** add or modify any subcommands or flags.

The CLI binary is unchanged by this guide. For the full flag reference, exit
codes, and output-format semantics see the [Rust SDK CLI reference](../../crates/sdk-rust/README.md).

## Running the CLI

From the workspace root, every command below is prefixed with:

```bash
cargo run -p stellarroute-sdk --bin stellarroute --
```

### Global flags (apply to every subcommand)

| Flag | Default | Description |
|---|---|---|
| `--api-url <URL>` | `http://127.0.0.1:3000` | Base URL for the StellarRoute API |
| `--output <human\|table\|json>` | `human` | Output format |

`STELLARROUTE_API_URL` is an alternative environment variable that sets the API
base URL, equivalent to `--api-url`.

## health

Probe the API and its dependencies (database, caches, etc.).

```bash
# Human-readable (default)
cargo run -p stellarroute-sdk --bin stellarroute -- health

# Table format
cargo run -p stellarroute-sdk --bin stellarroute -- --output table health

# JSON format (machine-friendly)
cargo run -p stellarroute-sdk --bin stellarroute -- --output json health

# Against a specific API instance
cargo run -p stellarroute-sdk --bin stellarroute -- --api-url http://127.0.0.1:3000 health
```

The `health` subcommand exits with code `0` when the API responds and reports
its status, even if a dependency reports `unhealthy`. Use the `status` and per-
component values in the output to triage.

## pairs

List the active trading pairs served by the API.

```bash
# First 10 pairs (default limit), human output
cargo run -p stellarroute-sdk --bin stellarroute -- pairs

# More pairs
cargo run -p stellarroute-sdk --bin stellarroute -- pairs --limit 50

# Table output
cargo run -p stellarroute-sdk --bin stellarroute -- --output table pairs --limit 20

# JSON output for scripting
cargo run -p stellarroute-sdk --bin stellarroute -- --output json pairs --limit 100

# Pairs served by a different API
cargo run -p stellarroute-sdk --bin stellarroute -- --api-url http://127.0.0.1:3000 --output json pairs
```

### Flags

| Flag | Default | Description |
|---|---|---|
| `--limit <N>` | `10` | Maximum number of pairs to print |

## quote

Fetch a price quote for a trading pair.

```bash
# Indicative sell quote (no amount -> server defaults to 1 unit)
cargo run -p stellarroute-sdk --bin stellarroute -- quote native USDC

# Sell 100 base units (XLM) for USDC
cargo run -p stellarroute-sdk --bin stellarroute -- quote native USDC --amount 100

# Buy 50 base units, JSON output
cargo run -p stellarroute-sdk --bin stellarroute -- --output json quote native USDC --amount 50 --quote-type buy

# Issued assets: CODE or CODE:ISSUER
cargo run -p stellarroute-sdk --bin stellarroute -- quote USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN USDC --amount 25 --output table
```

### Flags

| Flag | Default | Description |
|---|---|---|
| `--amount <decimal>` | _(omitted)_ | Positive decimal amount of the base asset to trade, e.g. `100` or `0.5`. Omit for an indicative 1-unit price. |
| `--quote-type <sell\|buy>` | `sell` | `sell` — trade away the base asset; `buy` — acquire the base asset. Maps to `quote_type` on `GET /api/v1/quote`. |

The base and quote assets accept `native`, `CODE`, or `CODE:ISSUER`. Slippage
tolerance (`slippage_bps`) is enforced server-side and is not a CLI flag.

## orderbook

Show the orderbook snapshot for a trading pair.

```bash
# 10 levels per side (default), human output
cargo run -p stellarroute-sdk --bin stellarroute -- orderbook native USDC

# More levels per side
cargo run -p stellarroute-sdk --bin stellarroute -- orderbook native USDC --levels 25

# Table output
cargo run -p stellarroute-sdk --bin stellarroute -- --output table orderbook native USDC --levels 20

# JSON output for scripting
cargo run -p stellarroute-sdk --bin stellarroute -- --output json orderbook native USDC --levels 5

# Issued asset pair
cargo run -p stellarroute-sdk --bin stellarroute -- orderbook USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN native --levels 10
```

### Flags

| Flag | Default | Description |
|---|---|---|
| `--levels <N>` | `10` | Maximum number of levels to print per side |

## Combined example

Chain global flags with a generous limit to script against a live instance:

```bash
API=http://127.0.0.1:3000
cargo run -p stellarroute-sdk --bin stellarroute -- --api-url "$API" health
cargo run -p stellarroute-sdk --bin stellarroute -- --api-url "$API" --output json pairs --limit 200
cargo run -p stellarroute-sdk --bin stellarroute -- --api-url "$API" --output json quote native USDC --amount 100 --quote-type sell
cargo run -p stellarroute-sdk --bin stellarroute -- --api-url "$API" --output json orderbook native USDC --levels 20
```

## Output formats

- `human` — friendly terminal output
- `table` — text table output
- `json` — machine-readable JSON

## Exit codes

- `0` — success
- `2` — CLI usage/validation error
- `3` — invalid client configuration
- `4` — runtime/API error

## Related documentation

- [Rust SDK CLI reference](../../crates/sdk-rust/README.md)
- [Rust SDK integration guide](README.md)
- [API error taxonomy](../api/error_taxonomy.md)
