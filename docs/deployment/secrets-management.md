# Secrets Management

This document is the single source of truth for **where every production
secret lives, who consumes it, and how it is rotated**. It covers Horizon /
Soroban RPC configuration, admin API tokens, integrator API keys, webhook
secrets, deployer keys, and datastore credentials.

Related documents:

- [`docs/key_rotation.md`](../key_rotation.md) — step-by-step rotation
  procedures for API keys, admin tokens, and webhook signing secrets.
- [`deploy/secrets.checklist.md`](../../deploy/secrets.checklist.md) —
  operator checklist to work through before the first deploy.
- [`docs/deployment/README.md`](./README.md) — deployment runbook, including
  the deployer key management and secret rotation checklist sections.

## Principles

1. **No secrets in git.** Secret material (tokens, keys, seed phrases,
   connection strings with passwords) is never committed — not in source, not
   in config files, not in deployment artifacts. `.gitignore` excludes
   `.env`, `.env.prod`, `.soroban/`, `*.secret-key`, and `identity.toml`, and
   the deploy artifact writer emits only non-secret fields.
2. **One secret store per environment.** Each runtime environment has exactly
   one authoritative secret store (see the inventory below). Copies elsewhere
   (developer machines, CI caches) are prohibited.
3. **Separate secrets per environment.** Testnet and mainnet never share
   deployer keys, admin tokens, or API keys. Staging never reuses production
   values.
4. **Least privilege.** CI jobs and services only receive the secrets they
   need. The indexer does not get `ADMIN_AUTH_TOKEN`; the API does not get
   the deployer secret key.
5. **Rotation is routine.** Every secret has an owner and a rotation
   procedure that works without downtime. Rotation is exercised on a
   schedule, not only after an incident.

## Secret stores by environment

| Environment | Authoritative store | Notes |
|---|---|---|
| Production / staging (Render) | Render dashboard → service → **Environment** (secret env vars / secret files) | `sync: false` keys in `render.yaml` are injected here at deploy time; never written to the repo |
| Production (self-hosted Compose) | `.env.prod` on the host, permissions `600`, never committed | Passed via `--env-file`; see `deploy/secrets.checklist.md` |
| CI/CD (GitHub Actions) | GitHub **repository secrets** (`Settings → Secrets and variables → Actions`) | Non-secret toggles use repository *variables* (`vars.*`); secret material uses `secrets.*` only |
| Local development | Developer-local `.env` (gitignored), based on `.env.example` | `.env.example` contains placeholders only, never real values |
| Consumer webhook signing secrets | Postgres table `consumer_quote_expiration_webhooks` | Managed per consumer via the admin API, not via env vars |

## Production secret inventory

Every production secret, its consumer, its store, and its rotation procedure:

| Secret | Consumed by | Store | Rotation procedure |
|---|---|---|---|
| `DATABASE_URL` (Postgres credentials) | API, indexer | Render (auto-wired from managed Postgres) / `.env.prod` | [Secret Rotation Checklist](./README.md#secret-rotation-checklist) |
| `REDIS_URL` / `REDIS_PASSWORD` | API | Render (auto-wired from managed Redis) / `.env.prod` | [Secret Rotation Checklist](./README.md#secret-rotation-checklist) |
| `API_KEYS` (integrator API keys) | API | Render env / `.env.prod` | [`docs/key_rotation.md` — API keys](../key_rotation.md#rotating-integrator-api-keys-api_keys) |
| `ADMIN_AUTH_TOKEN` (admin/operator token) | API (`/api/v1/admin/*`, `/api/v1/system/*`, production `/metrics` + `/api/v1/replay/*`) | Render env / `.env.prod` | [`docs/key_rotation.md` — admin token](../key_rotation.md#rotating-the-admin-token-admin_auth_token) |
| Webhook signing secrets (per consumer) | API webhook dispatcher (HMAC-SHA256 `x-stellarroute-signature`) | Postgres `consumer_quote_expiration_webhooks.signing_secret` | [`docs/key_rotation.md` — webhook secrets](../key_rotation.md#rotating-webhook-signing-secrets) |
| `LIQUIDITY_THINNESS_ALERT_WEBHOOK_URL` | API alerting | Render env / `.env.prod` | Issue a new webhook URL at the receiver, update the env var, redeploy, revoke the old URL |
| `TTL_ALERT_WEBHOOK_URL` | TTL monitoring scripts | Render env / cron host env | Same as above |
| `STELLAR_HORIZON_URL` (incl. any keyed/paid Horizon endpoint) | Indexer | Render env / `.env.prod` | [`docs/key_rotation.md` — Horizon/RPC](../key_rotation.md#rotating-horizon--soroban-rpc-credentials) |
| `SOROBAN_RPC_URL` (incl. any provider API key in the URL) | API (optional), indexer | Render env / `.env.prod` | [`docs/key_rotation.md` — Horizon/RPC](../key_rotation.md#rotating-horizon--soroban-rpc-credentials) |
| `SOROBAN_DEPLOYER_SECRET` (testnet deployer key) | CI deploy workflow | GitHub Actions repository secret | [Deployer key rotation](../key_rotation.md#rotating-deployer-keys) |
| `SOROBAN_MAINNET_DEPLOYER_SECRET` (mainnet deployer key) | CI mainnet deploy workflow | GitHub Actions repository secret | [Deployer key rotation](../key_rotation.md#rotating-deployer-keys) |
| Router **admin key** (on-chain admin) | Contract operations | Hardware wallet or encrypted vault — never in env vars or CI | On-chain `set_admin()` / multisig governance; see `audit/assumptions.md` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` (if the collector URL embeds a token) | API, indexer | Render env / `.env.prod` | Issue new collector token, update env var, redeploy |

Non-secret deployment configuration (`SOROBAN_CONTRACT_ID`, `DEPLOY_ENABLED`,
`DEPLOY_MAINNET_ENABLED`, `STELLARROUTE_ENV`, rate-limit knobs, etc.) is kept
in GitHub repository **variables** or plain env vars, so the secret store
stays small and auditable.

## CI/CD policy (GitHub Actions)

- **Secrets come from the platform, never from committed files.** Workflows
  reference `${{ secrets.* }}` exclusively; there are no encrypted blobs,
  `.env` files, or key files checked into the repository. This is asserted in
  review for every workflow change.
- **Secrets never appear in argv or logs.** The deploy workflows pipe
  `SOROBAN_DEPLOYER_SECRET` to `soroban keys add --secret-key stdin` so the
  key never hits process arguments or the run log. GitHub additionally masks
  registered secret values in workflow output.
- **Short-lived credentials via OIDC where supported.** For any future cloud
  integrations (e.g. AWS/GCP object storage for audit-log export, container
  registries), prefer GitHub's OIDC federation (`permissions: id-token:
  write` + the provider's official auth action) over long-lived static keys,
  so CI holds no persistent cloud credentials at all. Static repository
  secrets are the fallback only when the target platform (e.g. Render,
  Soroban deployer identities) does not support OIDC federation.
- **Least-privilege workflow tokens.** Workflows use the default read-only
  `GITHUB_TOKEN` unless a job explicitly needs more (e.g.
  `packages: write` for pushing Docker images).
- **Environment gating.** Mainnet deploys are gated behind the
  `DEPLOY_MAINNET_ENABLED` repository variable and a manual
  `workflow_dispatch` trigger, and use a dedicated mainnet-only secret.

## Handling a leaked secret

1. **Rotate immediately** using the matching procedure from the inventory
   table above — do not wait for a deploy window. All documented rotations
   are zero-downtime.
2. **Revoke the leaked value** at the source (delete the GitHub secret /
   Render env var, deregister the API key, disable the webhook registration,
   move funds off a leaked deployer account and retire it).
3. **Purge from history if committed.** If a secret was ever committed,
   rotation is mandatory even after a history rewrite — treat the value as
   permanently public.
4. **Audit for use.** Check API logs (admin endpoints are covered by the
   admin audit log), Render deploy history, and on-chain activity of deployer
   accounts for unauthorized use during the exposure window.
5. **Record the incident** as a GitHub issue labelled `security` with the
   exposure window, blast radius, and remediation steps taken.

## Verification

```bash
# Rotation and secret-store docs exist and cross-reference each other
rg -n 'rotat|secret' docs/key_rotation.md docs/deployment

# No committed .env files with real values
git ls-files | rg '^\.env$|\.env\.prod'   # should print nothing

# Workflows only consume platform secrets
rg -n 'secrets\.' .github/workflows
```
