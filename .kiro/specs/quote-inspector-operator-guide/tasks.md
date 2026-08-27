# Implementation Plan: Quote Inspector Operator Guide

## Overview

Two files are created: `docs/runbooks/quote-inspector.md` (the runbook) and a new proptest module in `crates/api/src/models/` that verifies the runbook stays accurate as the codebase evolves. No production code is touched.

## Tasks

- [x] 1. Write docs/runbooks/quote-inspector.md
  - Create the file with the section layout defined in the design: Overview → QuoteResponse Field Reference (UI mapping table, PathStep, rationale, exclusion diagnostics, timestamp fields) → AMM Warning → Prepare/Sign/Submit Flow → Reading Raw Quote JSON → Related Resources
  - Section 2.1: Markdown table with columns `UI Label | OpenAPI Field | Type | Notes` covering all 13 field pairs from requirement 2.1
  - Section 2.2: Secondary table for PathStep sub-fields (`from_asset`, `to_asset`, `price`, `source`, `liquidity_depth`, `fee_bps`)
  - Section 2.3: Prose + sub-table for `rationale` (`strategy`, `selected_source`, `compared_venues`); note it requires `X-Explain: true`
  - Section 2.4: Table of all five `ExclusionReason` variants with plain-English descriptions
  - Section 2.5: Table distinguishing `timestamp`, `source_timestamp`, `expires_at`, `ttl_seconds`
  - Section 3: `> **Warning:**` blockquote stating HTTP 422 / `unsupported_execution_mode` for AMM routes and the SDEX-only eligibility rule; describe `execution_mode: "classic_path_payment"` confirmation
  - Section 4: Numbered list of prepare → sign → submit steps with constraint bullets (no mutation, `network_passphrase` check, `expires_at` / `quote_expired`, `already_submitted` / 409)
  - Section 5: `ApiResponse` envelope table (`v`, `timestamp`, `request_id`, `data`); annotated `jsonc` code block for a single-hop SDEX quote + companion legend; degraded-quote diagnostics guidance
  - Section 6: Links to `/api-docs`, `docs/api/openapi.yaml`, `docs/api/error_taxonomy.md`, `docs/api/integrator-guide.md`, `docs/runbooks/swap-submitting-sender-lock.md`
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 5.1, 5.2, 5.3, 5.4, 6.1_

- [x] 2. Add proptest file for runbook accuracy
  - Create `crates/api/src/models/quote_inspector_props.rs` with four `#[cfg(test)]` proptest tests
  - Each test tagged with `// Feature: quote-inspector-operator-guide, Property N: ...` as specified in the design
  - Declare the module in `crates/api/src/models/mod.rs` under `#[cfg(test)]`

  - [x] 2.1 Implement Property 1 test — documented field names exist in schema
    - Tag: `// Feature: quote-inspector-operator-guide, Property 1: all documented QuoteResponse field names exist in the schema`
    - Iterate the 13 field names from requirement 2.1 (`amount`, `total`, `price`, `quote_type`, `base_asset`, `quote_asset`, `path`, `expires_at`, `data_freshness`, `degraded`, `price_impact`, `midpoint`, `spread_bps`)
    - Read `docs/api/openapi.yaml` (relative from workspace root) and assert each field name appears in the file content
    - _Requirements: 2.1_

  - [ ]* 2.2 Write property test for Property 1 using proptest harness
    - **Property 1: All documented QuoteResponse field names exist in the schema**
    - **Validates: Requirements 2.1**

  - [x] 2.3 Implement Property 2 test — all ExclusionReason variants are documented
    - Tag: `// Feature: quote-inspector-operator-guide, Property 2: all ExclusionReason variants are documented`
    - Hardcode the known variants (`policy_threshold`, `override`, `stale_data`, `circuit_breaker_open`, `liquidity_anomaly`)
    - Read `docs/runbooks/quote-inspector.md` and assert each variant string appears in the file content
    - _Requirements: 2.4_

  - [ ]* 2.4 Write property test for Property 2 using proptest harness
    - **Property 2: All ExclusionReason variants are documented**
    - **Validates: Requirements 2.4**

  - [x] 2.5 Implement Property 3 test — ApiResponse envelope fields are mentioned
    - Tag: `// Feature: quote-inspector-operator-guide, Property 3: all ApiResponse envelope fields are mentioned`
    - Iterate `["v", "timestamp", "request_id", "data"]`
    - Assert each appears in `docs/runbooks/quote-inspector.md`
    - _Requirements: 5.2_

  - [ ]* 2.6 Write property test for Property 3 using proptest harness
    - **Property 3: All ApiResponse envelope fields are mentioned**
    - **Validates: Requirements 5.2**

  - [x] 2.7 Implement Property 4 test — key flow constraint identifiers are present
    - Tag: `// Feature: quote-inspector-operator-guide, Property 4: all key flow constraint identifiers are present`
    - Iterate `["quote_expired", "already_submitted", "unsupported_execution_mode", "network_passphrase", "signed_xdr", "classic_path_payment"]`
    - Assert each appears in `docs/runbooks/quote-inspector.md`
    - _Requirements: 3.2, 3.4, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_

  - [ ]* 2.8 Write property test for Property 4 using proptest harness
    - **Property 4: All key flow constraint identifiers are present**
    - **Validates: Requirements 3.2, 3.4, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**

- [x] 3. Final checkpoint
  - Ensure all tests pass, ask the user if questions arise.
  - Verify `cargo test -p stellarroute-api --lib` is green
  - Verify no production files under `crates/api/src/routes/` were modified

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- The only files this feature touches are `docs/runbooks/quote-inspector.md` and `crates/api/src/models/quote_inspector_props.rs` (plus the `mod.rs` declaration)
- Property tests 1–4 are deterministic set-membership checks; `proptest` is used as the harness per the design's testing strategy
- All four properties run a minimum of 100 iterations as specified in the design
