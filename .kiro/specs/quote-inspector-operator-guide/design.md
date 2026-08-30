# Design Document: Quote Inspector Operator Guide

## Overview

This feature is a single new markdown file: `docs/runbooks/quote-inspector.md`. There is no production code, no API change, no feature flag, and no schema modification. The deliverable is the runbook itself.

The design document describes:
- the section layout and content approach for the runbook,
- how to present each piece of content (tables, blockquotes, code blocks, prose),
- cross-references to existing docs and source files,
- correctness properties that verify the runbook stays accurate as the codebase evolves.

## Architecture

The runbook lives entirely in `docs/runbooks/`. It is a Markdown file consumed by any operator, contributor, or on-call engineer. No build pipeline processes it; it is rendered by GitHub and any static-site tooling already applied to the `docs/` tree.

```
docs/
  runbooks/
    quote-inspector.md   ← new file (only change)
    router-indexer-runbook.md
    swap-submitting-sender-lock.md
```

Cross-references point to:
- `docs/api/openapi.yaml` — canonical field schema
- `/api-docs` — live Swagger UI
- `docs/api/error_taxonomy.md` — error code reference
- `docs/api/integrator-guide.md` — integrator webhook/idempotency details
- `docs/runbooks/swap-submitting-sender-lock.md` — prepare/submit sender-lock recovery

## Components and Interfaces

### Runbook Section Layout

```
docs/runbooks/quote-inspector.md
├── 1. Overview
├── 2. QuoteResponse Field Reference
│   ├── 2.1 UI → API Field Mapping Table
│   ├── 2.2 PathStep Sub-fields
│   ├── 2.3 Rationale (explain mode)
│   ├── 2.4 Exclusion Diagnostics
│   └── 2.5 Timestamp Fields
├── 3. AMM Routes: Current Limitation
├── 4. Prepare → Sign → Submit Flow
├── 5. Reading Raw Quote JSON
│   ├── 5.1 ApiResponse Envelope
│   ├── 5.2 Annotated Single-hop SDEX Example
│   └── 5.3 Diagnosing a Degraded Quote
└── 6. Related Resources
```

### Section Content Decisions

**Section 1 — Overview**
Prose paragraph: why the endpoint exists, what the runbook covers, and a pointer to `/api-docs` for live schema. Keep to 3–5 sentences.

**Section 2.1 — UI → API Mapping Table**
Markdown table with four columns: `UI Label | OpenAPI Field | Type | Notes`. The extra Type column is low-cost and helps operators quickly understand whether a field is a string, boolean, or nested object without opening the schema. All 13 required field pairs from requirement 2.1 are included.

**Section 2.2 — PathStep Sub-fields**
Secondary table listing `from_asset`, `to_asset`, `price`, `source`, `liquidity_depth`, `fee_bps` with types and notes on which are optional. This avoids requiring operators to follow the OpenAPI link just to understand a hop.

**Section 2.3 — Rationale**
Short prose paragraph explaining `rationale` is only present when `X-Explain: true` is sent (or `?explain=true`), followed by an inline sub-table for `strategy`, `selected_source`, `compared_venues`.

**Section 2.4 — Exclusion Diagnostics**
Prose introduction followed by a table of all five `ExclusionReason` variants (`policy_threshold`, `override`, `stale_data`, `circuit_breaker_open`, `liquidity_anomaly`) with a plain-English description of each.

**Section 2.5 — Timestamp Fields**
Short table distinguishing `timestamp` (ms, when the quote was generated), `source_timestamp` (ms, age of the underlying market data), `expires_at` (ms, client-side staleness deadline), and `ttl_seconds` (seconds, convenience duplicate of the expiry window).

**Section 3 — AMM Warning**
A Markdown blockquote admonition (`> **Warning:**`) at the top of the section, followed by prose. The blockquote states the HTTP 422 / `unsupported_execution_mode` behavior and the eligibility rule. Then a brief description of what `execution_mode: "classic_path_payment"` confirms.

Rationale for blockquote over admonition syntax: GitHub renders `>` blockquotes on all views; admonition syntax (`> [!WARNING]`) renders on GitHub.com but not in all static-site renderers the project may adopt. Using `> **Warning:**` is portable.

**Section 4 — Prepare → Sign → Submit**
Numbered list for the three steps (not prose paragraphs), because operators scan numbered lists under time pressure. Each step names the endpoint, the key field produced/consumed, and any constraint. Constraints (no mutation, network passphrase check, expiry, already-submitted) are bullet points beneath the relevant step.

**Section 5.2 — Annotated JSON Example**
Inline comments (`// ...`) inside a JSON code block, with a companion legend table below. JSON does not support comments by spec, but the code block is labeled `jsonc` so operators understand it is illustrative. The legend table reuses the same field names from section 2.1 for consistency; operators can cross-reference without scanning.

Rationale for inline-comments + legend (not prose walkthrough): the example is the primary reference artifact. Inline comments give context at the point of reading; the legend gives a scannable summary for operators who already know the shape.

## Data Models

The runbook documents the following live data shapes exactly as they exist in `crates/api/src/models/response.rs` and `docs/api/openapi.yaml`. No new models are introduced.

### QuoteResponse (summary)

| Field | Type | Optional |
|---|---|---|
| `base_asset` | `AssetInfo` | no |
| `quote_asset` | `AssetInfo` | no |
| `amount` | string | no |
| `price` | string | no |
| `total` | string | no |
| `quote_type` | `"sell"` \| `"buy"` | no |
| `degraded` | boolean | no (default `false`) |
| `path` | `PathStep[]` | no |
| `timestamp` | int (ms) | no |
| `expires_at` | int (ms) | yes |
| `source_timestamp` | int (ms) | yes |
| `ttl_seconds` | int | yes |
| `rationale` | `QuoteRationaleMetadata` | yes |
| `price_impact` | string | yes |
| `exclusion_diagnostics` | `ExclusionDiagnostics` | yes |
| `data_freshness` | `DataFreshness` | yes |
| `midpoint` | string | yes |
| `spread_bps` | int | yes |

### PathStep

| Field | Type | Optional |
|---|---|---|
| `from_asset` | `AssetInfo` | no |
| `to_asset` | `AssetInfo` | no |
| `price` | string | no |
| `source` | string (`"sdex"` or `"amm:<pool>"`) | no |
| `liquidity_depth` | string | yes |
| `fee_bps` | int | yes |

### SwapPrepareResponse

| Field | Type | Notes |
|---|---|---|
| `quote_id` | string | Used in submit |
| `xdr_envelope` | string | Unsigned XDR; wallet signs this |
| `expected_output` | string | Expected receive amount |
| `min_output` | string | Optional; slippage floor |
| `expires_at` | int (ms) | Submit before this or get 422 `quote_expired` |
| `execution_mode` | string | Always `"classic_path_payment"` today |
| `network_passphrase` | string | Must match wallet network |

### ApiResponse Envelope

| Field | Notes |
|---|---|
| `v` | Schema version integer (currently `1`) |
| `timestamp` | Unix ms when the response was generated |
| `request_id` | Correlation ID (echoes `X-Request-ID` or server-generated) |
| `data` | The `QuoteResponse` object |

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

For a docs-only feature, the meaningful correctness questions are: does the document accurately reflect the live code and schema? The following properties express that as verifiable claims, testable against `docs/api/openapi.yaml` and `crates/api/src/models/response.rs`.

---

### Property 1: All documented QuoteResponse field names exist in the schema

*For any* field name listed in the runbook's UI→API mapping table (section 2.1), that field name SHALL appear as a key in the `QuoteResponse` schema in `docs/api/openapi.yaml` or in the `QuoteResponse` struct in `crates/api/src/models/response.rs`.

Reasoning: the table in requirement 2.1 enumerates 13 specific field names. As the codebase evolves, fields could be renamed or removed. A property test that iterates the documented set and checks each against the live schema catches drift before it confuses operators.

**Validates: Requirements 2.1**

---

### Property 2: All ExclusionReason variants are documented

*For any* `ExclusionReason` variant defined in `crates/api/src/models/response.rs`, the lowercased snake_case name of that variant SHALL appear somewhere in the runbook.

Reasoning: new exclusion reasons may be added over time. A test that reads the source enum and checks each variant against the doc text catches newly-added reasons that the runbook hasn't caught up with.

**Validates: Requirements 2.4**

---

### Property 3: All ApiResponse envelope fields are mentioned in the runbook

*For any* top-level field of the `ApiResponse` wrapper (`v`, `timestamp`, `request_id`, `data`), that field name SHALL appear in the runbook text.

Reasoning: the envelope shape is relied on by all callers. Documenting only three of four fields would leave operators confused when they encounter the fourth.

**Validates: Requirements 5.2**

---

### Property 4: All key flow constraint identifiers are present in the runbook

*For any* item in the set of required identifiers — `quote_expired`, `already_submitted`, `unsupported_execution_mode`, `network_passphrase`, `signed_xdr`, `classic_path_payment` — that string SHALL appear in the runbook text.

Reasoning: these strings are the exact error codes and field names operators paste into their tooling. If any one is missing or misspelled the guide loses its utility for the described scenario. A single property over the complete set is more maintainable than six separate checks.

**Validates: Requirements 3.2, 3.4, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**

## Error Handling

The runbook is a static document; it has no runtime error handling. Its accuracy is maintained through:

1. A CI lint step (see Testing Strategy) that checks field name presence against the schema.
2. A manual review gate — changes to `QuoteResponse`, `PathStep`, `SwapPrepareResponse`, or `ExclusionReason` in `response.rs` should be accompanied by a runbook update.
3. The runbook links to `/api-docs` so operators can cross-check the live schema.

## Testing Strategy

Because this is a docs-only change, no unit tests apply to the runbook content itself. Testing is split across:

**Unit tests (example-based)**
Located in `docs/` validation scripts or inline Rust doc tests. Verify:
- The file `docs/runbooks/quote-inspector.md` exists.
- The strings `/api-docs`, `unsupported_execution_mode`, `classic_path_payment`, `quote_expired`, `already_submitted`, `network_passphrase`, `signed_xdr` all appear in the file.
- The blockquote warning block is present (file contains `> **Warning**`).
- All four `ApiResponse` envelope field names (`"v"`, `"timestamp"`, `"request_id"`, `"data"`) appear in the file.

**Property-based tests**
Use the `proptest` crate (already present in the workspace). Tests live alongside existing model tests, e.g. in `crates/api/src/models/response.rs`.

Each property test runs a minimum of 100 iterations (though properties 1–4 above are deterministic set-membership checks; the iteration count applies to any randomized generator used for the input side).

Tag format: `// Feature: quote-inspector-operator-guide, Property {N}: {property_text}`

- Property 1 test: reads `docs/api/openapi.yaml` and `crates/api/src/models/response.rs`, iterates over the 13 documented field names, asserts each appears in the schema.
  Tag: `// Feature: quote-inspector-operator-guide, Property 1: all documented QuoteResponse field names exist in the schema`

- Property 2 test: parses the `ExclusionReason` enum variants from source, reads `docs/runbooks/quote-inspector.md`, asserts each variant's snake_case name appears.
  Tag: `// Feature: quote-inspector-operator-guide, Property 2: all ExclusionReason variants are documented`

- Property 3 test: iterates `["v", "timestamp", "request_id", "data"]`, asserts each appears in the runbook.
  Tag: `// Feature: quote-inspector-operator-guide, Property 3: all ApiResponse envelope fields are mentioned`

- Property 4 test: iterates `["quote_expired", "already_submitted", "unsupported_execution_mode", "network_passphrase", "signed_xdr", "classic_path_payment"]`, asserts each appears in the runbook.
  Tag: `// Feature: quote-inspector-operator-guide, Property 4: all key flow constraint identifiers are present`

**What the tests intentionally do not cover**
- Prose quality, completeness of explanations, or whether the blockquote "feels" prominent — these are human review concerns.
- The additive-only constraint (Requirement 6) is enforced by the PR diff review and CI, not by doc-content tests.

**Property test library**: `proptest` (`crates/api/Cargo.toml` already includes it).
**Minimum iterations**: 100 per property test.
