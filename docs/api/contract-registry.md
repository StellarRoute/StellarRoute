# Contract Registry API

The contract registry is a read-only metadata endpoint for integrators that need to discover which deployed contract version and WASM hash is associated with a Stellar network. It is useful for version checks, deployment verification, and SDK or client compatibility checks.

The registry is not part of live swap execution. It does not prepare, sign, submit, or confirm transactions.

## List registered contracts

```http
GET /api/v1/contracts/registry
```

The response is a JSON array ordered by `deployed_at` descending. Each item has the following fields:

| Field | Type | Description |
|---|---|---|
| `contract_name` | string | Contract name or identifier. |
| `version` | string | Semantic version, such as `1.2.3`. |
| `wasm_hash` | string | Hex-encoded WASM hash. |
| `network` | string | Network identifier, such as `mainnet`, `testnet`, or `futurenet`. |
| `contract_address` | string or `null` | Deployed contract address, when available. |
| `deployed_at` | integer or `null` | Deployment time as a Unix timestamp in seconds. |
| `git_commit` | string or `null` | Git commit SHA used to build the version, when available. |

Example response:

```json
[
  {
    "contract_name": "stellar_router",
    "version": "1.2.3",
    "wasm_hash": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    "network": "testnet",
    "contract_address": "C...",
    "deployed_at": 1700000000,
    "git_commit": "abc1234"
  }
]
```

Optional fields remain present as JSON `null` when no value is registered.

## Get the latest version for a contract

```http
GET /api/v1/contracts/registry/{contract_name}
```

This returns the latest registered row for `contract_name`, ordered by deployment time. The contract name is a path segment and must be URL-encoded when necessary.

## Get the latest version for a contract on a network

```http
GET /api/v1/contracts/registry/{contract_name}/network/{network}
```

This returns the latest registered row matching both `contract_name` and `network`. The network is a path segment, for example `testnet`.

## Errors and integration guidance

The lookup endpoints return `404 Not Found` with the API error response when no matching contract is registered. The list endpoint returns `200 OK` with an empty array when there are no registered rows. Database or other server failures return `500 Internal Server Error`.

Treat registry data as deployment metadata. Use the normal quote and swap endpoints for current market pricing and transaction execution, and do not use a registry response as a substitute for a live quote.
