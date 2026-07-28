# deploy/secrets.checklist.md
# StellarRoute — Staging Secrets Checklist (Issue #1035)
#
# Work through this list before deploying.  All items marked ✅ must be in place
# before the first `up` / deploy.

## Render deploy

- [ ] `DATABASE_URL` — automatically wired from `stellarroute-postgres` by `render.yaml`
- [ ] `REDIS_URL` — automatically wired from `stellarroute-redis` by `render.yaml`
- [ ] `SOROBAN_RPC_URL` — set in Render dashboard → Environment → Secret Files (e.g. `https://soroban-rpc.testnet.stellar.org`)
- [ ] `STELLAR_HORIZON_URL` — set in Render dashboard (e.g. `https://horizon-testnet.stellar.org`)
- [ ] `ROUTER_CONTRACT_ADDRESS` — set in Render dashboard (deployed contract ID)
- [ ] `ENABLE_ADMIN_ROUTES` — leave as `false` until security issues are resolved (see README §Security)
- [ ] `OTEL_EXPORTER_OTLP_ENDPOINT` — optional; set if you have a collector

## Docker Compose (production overlay)

Create `.env.prod` in the repo root (never commit it):

```env
POSTGRES_USER=stellarroute
POSTGRES_PASSWORD=<strong-random-password>
POSTGRES_DB=stellarroute
REDIS_PASSWORD=<strong-random-password>
SOROBAN_RPC_URL=https://soroban-rpc.testnet.stellar.org
STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org
ROUTER_CONTRACT_ADDRESS=<deployed-contract-id>
ENABLE_ADMIN_ROUTES=false
# OTEL_EXPORTER_OTLP_ENDPOINT=http://your-collector:4318
```

Then:

```bash
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod up -d
```

## Post-deploy verification

```bash
# API liveness
curl -sf https://<your-render-url>/health && echo "API live"

# API readiness (checks Postgres + Redis)
curl -sf https://<your-render-url>/health/deps && echo "API ready"
```
