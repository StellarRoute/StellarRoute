# Issue #1282: Liquidity Thinness Alert Regression Coverage

## Why

`crates/api/tests/liquidity_thinness_alerts_integration.rs` exists, but it does not verify that a thin orderbook fixture emits the documented alert signal while the quote HTTP response retains its current shape.

## Scope

Add an integration-test case using a thin fixture that:

- asserts the documented liquidity thinness alert signal fires;
- asserts the quote endpoint returns HTTP 200; and
- verifies the quote JSON field set is unchanged from the existing snapshot and tests.

This issue is additive-only. It must not change behavior for live users.

## Frozen Behavior

- Classic one-hop SDEX prepare, wallet sign, and submit flow in `crates/api/src/routes/swap.rs`.
- Quote selection and ranking in `crates/api/src/routes/quote.rs`.
- Wallet connect and sign adapters used by the live swap CTA.
- Existing OpenAPI field names, types, and error codes.
- CORS allowlists, the `CCTP_ENABLED` default, and real XDR pinning.
- `/swap` layout, header chrome, and primary CTA unless a new flag defaults false and is unset in production.

Do not rename or remove JSON fields, tighten lean CI, or enable flags in Vercel or `.env.example`.

## Allowed Files

- `crates/api/tests/liquidity_thinness_alerts_integration.rs`
- Relevant test fixtures.
- Optional comment in `docs/LIQUIDITY_THINNESS_ALERTS_RUNBOOK.md`.

## Acceptance Criteria

- A thin fixture triggers the documented alert signal.
- The quote HTTP 200 body has the unchanged JSON field set.
- Existing swap, quote, and OpenAPI contract tests pass unchanged.
- With flags unset, or with no new flag, `/swap` has no user-visible difference.
- The pull request description lists every production file touched and explains why each change is additive.

## Out of Scope

This issue does not duplicate open issues #1200 through #1279.