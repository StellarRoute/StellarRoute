# Secret & Key Rotation

This document contains the step-by-step rotation procedures for every
runtime secret. The full inventory of production secrets — where each one is
stored and who consumes it — lives in
[`docs/deployment/secrets-management.md`](deployment/secrets-management.md).
Deployer-key handling and the generic datastore rotation checklist are in
[`docs/deployment/README.md`](deployment/README.md#key-management).

All procedures below are zero-downtime: the new credential is introduced
alongside the old one, verified, and only then is the old one revoked.

## Rotating integrator API keys (`API_KEYS`)

API keys are provided via the `API_KEYS` environment variable as a
comma-separated list.

```bash
export API_KEYS="secret-key-1,secret-key-2"
export REQUIRE_AUTH="true"
```

To rotate an API key without downtime:

1. Append the new key to the `API_KEYS` environment variable:
   `API_KEYS="old-key,new-key"`
2. Restart the application servers or trigger a rolling deployment so the new key becomes active.
3. Update the client integration to use `new-key`.
4. Once the client has completely switched to `new-key`, remove `old-key` from `API_KEYS`:
   `API_KEYS="new-key"`
5. Perform another rolling deployment to revoke `old-key`.

## Rotating the admin token (`ADMIN_AUTH_TOKEN`)

`ADMIN_AUTH_TOKEN` gates `/api/v1/admin/*`, `/api/v1/system/*`, and (in
production) `/metrics` and `/api/v1/replay/*`. It is a single-value token, so
rotation is a swap rather than an append:

1. Generate a new high-entropy token (at least 32 random bytes):
   `openssl rand -base64 32`
2. Update `ADMIN_AUTH_TOKEN` in the secret store for the environment (Render
   dashboard env var, or `.env.prod` for Compose). Do **not** unset it — in
   production the API refuses to boot without it, and admin endpoints deny
   all requests while it is missing.
3. Trigger a rolling deployment. Instances pick up the new token at boot.
4. Update operator tooling (dashboards, scripts, on-call runbook secrets) to
   send the new token via `x-admin-token` or `Authorization: Bearer`.
5. Verify: a request with the old token must now receive `401 Unauthorized`,
   and a request with the new token must succeed. Check the admin audit log
   for any use of the old token after the rotation point.

Because admin mutations are audited (see `crates/api/src/admin_audit.rs`),
rotate immediately and review the audit log if the token may have leaked.

## Rotating webhook signing secrets

Outbound quote-expiration webhooks are signed with HMAC-SHA256 using a
**per-consumer** `signing_secret` stored in Postgres
(`consumer_quote_expiration_webhooks`), managed via
`POST /api/v1/integrator/webhooks/quote-expiration`. To rotate one consumer's
secret:

1. Generate a new signing secret: `openssl rand -hex 32`
2. Coordinate with the consumer: they configure their receiver to accept
   signatures from **both** the old and the new secret during the cutover.
3. Upsert the registration with the new secret (same `consumer_id` and
   `webhook_url`):

   ```bash
   curl -X POST "$API_URL/api/v1/integrator/webhooks/quote-expiration" \
     -H "x-api-key: $INTEGRATOR_KEY" \
     -H "content-type: application/json" \
     -d '{"consumer_id":"acme","webhook_url":"https://consumer.example/hook","signing_secret":"<new-secret>","enabled":true}'
   ```

4. Deliveries are signed with the new secret from the next dispatch onward
   (`x-stellarroute-signature` header). The consumer verifies a live delivery
   against the new secret, then drops the old secret from their receiver.
5. If the old secret leaked, disable the registration (`"enabled": false`)
   first, rotate, then re-enable.

Inbound alert webhooks (`LIQUIDITY_THINNESS_ALERT_WEBHOOK_URL`,
`TTL_ALERT_WEBHOOK_URL`) embed their credential in the URL. Rotate by issuing
a new webhook URL at the receiving service (Slack/PagerDuty/etc.), updating
the env var, redeploying, and revoking the old URL.

## Rotating Horizon / Soroban RPC credentials

`STELLAR_HORIZON_URL` and `SOROBAN_RPC_URL` may point at keyed or paid
provider endpoints where the access key is part of the URL or an associated
header. To rotate:

1. Issue a new key/endpoint at the provider while the old one is still valid.
2. Update the env var(s) in the secret store (Render dashboard / `.env.prod`).
3. Restart the indexer first, then the API, one instance at a time.
4. Confirm `GET /health/deps` stays healthy and the indexer resumes ingesting
   (offer/pool timestamps keep advancing).
5. Revoke the old provider key.

The public Stellar endpoints carry no credential; this section applies as
soon as a rate-limited or paid provider is used in production.

## Rotating deployer keys

The Soroban deployer secret keys live only in GitHub Actions repository
secrets (`SOROBAN_DEPLOYER_SECRET` for testnet,
`SOROBAN_MAINNET_DEPLOYER_SECRET` for mainnet — never shared between
networks). To rotate:

1. Generate a new identity: `soroban keys generate deployer-new --network <net>`
2. Fund the new account (Friendbot on testnet; XLM transfer on mainnet).
3. If the current deployer is the router admin, transfer contract admin to
   the new account (`set_admin()` / governance proposal) **before** retiring
   the old key.
4. Replace the GitHub repository secret with the new secret key.
5. Run the deploy workflow in dry-run mode to confirm the new identity works.
6. Move remaining funds off the old account and retire it.

## Rotation schedule

| Secret | Cadence | Also rotate when |
|---|---|---|
| `API_KEYS` entries | 90 days | An integrator off-boards or a key leaks |
| `ADMIN_AUTH_TOKEN` | 90 days | An operator off-boards; any suspected leak |
| Webhook signing secrets | 180 days | Consumer requests it; any suspected leak |
| Horizon / RPC provider keys | Provider default | Provider incident; any suspected leak |
| Deployer keys | Yearly | Any suspected compromise (rotate immediately) |
| Postgres / Redis credentials | 180 days | Any suspected leak |

Rotations are tracked as recurring GitHub issues labelled `security` so the
schedule is auditable.
