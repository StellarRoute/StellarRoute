# Runbook: Swap `submitting` sender-lock recovery

**Scope:** Classic Stellar SDEX `PathPaymentStrictSend` prepare/submit only.
**Out of scope:** Soroban / AMM / router execution (unsupported in this build —
do not attempt operator “recovery” that implies Soroban submit).

## Why this exists

Prepare reserves **one active quote per sender G-account**
(`prepared` or `submitting`). A quote that entered `submitting` with a bound
`tx_hash` is **never TTL-expired** by the API: it remains reconcilable until
Horizon shows a terminal outcome or an operator follows this procedure.

That means a stuck `submitting` row **indefinitely blocks** a new
`POST /api/v1/swap/prepare` for the same `sender_account`
(`409` / `active_prepare_exists`). Clearing the lock without Horizon
reconciliation can double-broadcast or abandon an already-accepted
transaction — treat that as a severity-1 mistake.

There is **no admin HTTP endpoint** for this recovery today. Operators use
read-only Horizon checks, then a **guarded SQL** update that mirrors
`SwapQuoteStore::mark_failed` (status → `failed` only from
`prepared`/`submitting`), plus a mandatory `swap_submit_audit_log` row.

## Preconditions

- Access to the API Postgres primary (read/write).
- Network access to the configured Horizon
  (`STELLAR_HORIZON_URL`, testnet default `https://horizon-testnet.stellar.org`).
- The stuck quote’s `quote_id`, `tx_hash`, `sender_account`, and
  `timebounds_max` (Unix seconds, Stellar tx `max_time`).

## Step 0 — Identify the lock

```sql
-- Replace :sender with the G-address (or use quote_id).
SELECT quote_id,
       sender_account,
       submission_status,
       tx_hash,
       expires_at,
       timebounds_max,
       source_sequence,
       network_passphrase,
       execution_mode
FROM swap_prepared_quotes
WHERE sender_account = :sender
  AND submission_status IN ('prepared', 'submitting')
ORDER BY expires_at DESC;
```

Expected for this runbook: `submission_status = 'submitting'` and
`tx_hash IS NOT NULL`.

If `submission_status = 'submitting'` **and** `tx_hash IS NULL`, treat as an
integrity incident (should not occur after claim-with-hash). Do **not**
clear blindly — escalate engineering; leave the row until investigated.

If `execution_mode` is anything other than `classic_path_payment`, stop:
this runbook does not cover Soroban/AMM.

## Step 1 — Reconcile the bound transaction hash (mandatory first)

Never mark failed before this step.

```bash
HORIZON="${STELLAR_HORIZON_URL:-https://horizon-testnet.stellar.org}"
TX_HASH="<tx_hash from Step 0>"

# 404 = Horizon has no record of this hash (yet).
# 200 = transaction exists — inspect successful / ledger / created_at.
curl -sS -o /tmp/horizon-tx.json -w "%{http_code}" \
  "${HORIZON%/}/transactions/${TX_HASH}"
echo
jq '{hash, successful, ledger, created_at}' /tmp/horizon-tx.json 2>/dev/null || true
```

| Horizon result | Action |
|---|---|
| **200**, `successful: true` (or pending inclusion) | **Do not mark failed.** Finalize operationally: set `submission_status = 'submitted'` if still `submitting` (SQL below), keep `tx_hash`, record audit `operator_reconcile_finalize`. Client may retry `submit` for idempotent success, or you apply the finalize SQL. |
| **200**, hash **≠** stored `tx_hash` | **Integrity mismatch.** Leave row `submitting`. See [Horizon hash-integrity mismatch](#horizon-hash-integrity-mismatch). |
| **404** | Proceed only after Step 2 (timebounds elapsed). Retry Horizon once more immediately before Step 3. |
| Transport / 5xx | Retry later; leave `submitting`. |

### Optional finalize when Horizon already accepted (same hash)

```sql
-- Only when Horizon 200 confirms THIS exact tx_hash.
BEGIN;

UPDATE swap_prepared_quotes
SET submission_status = 'submitted',
    submitted_at = COALESCE(submitted_at, NOW()),
    tx_hash = :tx_hash
WHERE quote_id = :quote_id
  AND submission_status = 'submitting'
  AND tx_hash = :tx_hash;

-- Expect exactly 1 row updated.
INSERT INTO swap_submit_audit_log (
    quote_id, tx_hash, account, request_id, trace_id,
    latency_ms, outcome, error_class, metadata
) VALUES (
    :quote_id,
    :tx_hash,
    :sender_account_hash,  -- redacted hash from row.sender_account_hash; NEVER raw G-address
    'operator-runbook',
    '',
    0,
    'submitted',
    'none',
    jsonb_build_object(
        'event', 'operator_reconcile_finalize',
        'operator', :operator_id,
        'horizon', 'found'
    )
);

COMMIT;
```

## Step 2 — Wait until transaction timebounds have elapsed

The unsigned/signed tx includes `Preconditions::Time` with
`max_time = timebounds_max` (Unix seconds stored on the quote).

```sql
SELECT quote_id,
       tx_hash,
       timebounds_max,
       to_timestamp(timebounds_max) AS timebounds_max_utc,
       NOW() AS db_now,
       (EXTRACT(EPOCH FROM NOW())::bigint > timebounds_max) AS timebounds_elapsed
FROM swap_prepared_quotes
WHERE quote_id = :quote_id;
```

**Rule:** Only continue to mark-failed / release when:

1. `timebounds_elapsed` is true (`NOW()` strictly after `timebounds_max`), **and**
2. Horizon still returns **404** for the bound `tx_hash` (re-check Step 1).

If timebounds have not elapsed, tell the user to retry
`POST /api/v1/swap/submit` with the **same** signed envelope (idempotent
reconcile path). Do not clear the sender lock.

## Step 3 — Guarded mark-failed (release sender lock)

**Warning:** Clearing a quote that Horizon may still accept (or has accepted)
can cause the client to prepare a new sequence and submit a second payment.
Only run this after Steps 1–2.

This SQL matches application `mark_failed`: transition to `failed` only from
`prepared`/`submitting`, and only when the bound hash is unchanged.

```sql
BEGIN;

-- Re-assert preconditions inside the transaction.
SELECT quote_id, submission_status, tx_hash, timebounds_max
FROM swap_prepared_quotes
WHERE quote_id = :quote_id
FOR UPDATE;

-- Abort in the client if:
--   submission_status <> 'submitting'
--   OR tx_hash IS DISTINCT FROM :tx_hash
--   OR timebounds_max IS NULL
--   OR EXTRACT(EPOCH FROM NOW())::bigint <= timebounds_max

UPDATE swap_prepared_quotes
SET submission_status = 'failed'
WHERE quote_id = :quote_id
  AND submission_status = 'submitting'
  AND tx_hash = :tx_hash
  AND timebounds_max IS NOT NULL
  AND EXTRACT(EPOCH FROM NOW())::bigint > timebounds_max;

-- Expect exactly 1 row updated. If 0, ROLLBACK and re-run Step 1.

INSERT INTO swap_submit_audit_log (
    quote_id, tx_hash, account, request_id, trace_id,
    latency_ms, outcome, error_class, metadata
) VALUES (
    :quote_id,
    :tx_hash,
    :sender_account_hash,  -- from sender_account_hash column; never log full account/XDR
    'operator-runbook',
    '',
    0,
    'failed',
    'operator_release_after_timebounds',
    jsonb_build_object(
        'event', 'operator_release_sender_lock',
        'operator', :operator_id,
        'horizon', 'absent_after_timebounds',
        'timebounds_max', :timebounds_max
    )
);

COMMIT;
```

After commit, the sender may call `prepare` again. Prefer a **fresh prepare**
(new sequence / timebounds) rather than reusing the old signed envelope.

## Horizon hash-integrity mismatch

If Horizon returns a transaction whose `hash` does not equal the quote’s
stored `tx_hash`, or the API logs an internal “horizon lookup hash mismatch”:

1. **Leave** `submission_status = 'submitting'` — do not mark failed, do not
   null `tx_hash`, do not delete the row.
2. Capture: `quote_id`, stored `tx_hash`, Horizon response hash/body
   (redact accounts), API request_id / deploy SHA, Horizon URL.
3. File an incident; engineering investigates store/Horizon inconsistency.
4. Sender remains locked until investigation concludes with an explicit
   finalize (Step 1 accepted path) or a post-mortem–approved release.

## What not to do

- Do **not** `DELETE` from `swap_prepared_quotes` to “unlock” a sender.
- Do **not** set `tx_hash = NULL` while leaving `submitting` (integrity hole).
- Do **not** `UPDATE … SET submission_status = 'prepared'` to revive a
  stuck submit (revives sequence/envelope races).
- Do **not** clear a lock because prepare TTL (`expires_at`) passed —
  TTL does not apply to `submitting`.
- Do **not** paste full accounts, secrets, or raw XDR into audit `metadata`
  or tickets.

## Related

- Live swap checklist: [`docs/readiness/live-swap-testnet-checklist.md`](../readiness/live-swap-testnet-checklist.md)
- Error taxonomy (swap): [`docs/api/error_taxonomy.md`](../api/error_taxonomy.md#swap-preparesubmit)
- Store behavior: `crates/api/src/swap/store.rs` (`claim_for_submit`, `mark_failed`, `expire_stale_for_sender`)
