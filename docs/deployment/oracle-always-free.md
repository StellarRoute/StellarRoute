# Oracle Always Free — public testnet API (Wave 0)

Free always-on hosting for StellarRoute API + indexer + Postgres + Redis on an
[Oracle Cloud Always Free](https://www.oracle.com/cloud/free/) ARM VM, with a
**named Cloudflare Tunnel** for public HTTPS (no inbound 80/443 on the VM).

Related: `deploy/docker-compose.prod.yml`, `deploy/secrets.checklist.md`,
`deploy/env.prod.example`, `scripts/oracle/bootstrap-vm.sh`,
`scripts/staging-smoke.sh`.

## Architecture

```
Internet → Cloudflare Tunnel (HTTPS) → 127.0.0.1:8080 (API container)
                                         ├─ Postgres (no host port)
                                         ├─ Redis (no host port)
                                         └─ Indexer → Horizon + Soroban RPC + router
```

Vercel frontend calls the tunnel hostname. Set `CORS_ALLOWED_ORIGINS` to
`https://www.stellarroute.app`, `https://stellarroute.app`, and the Vercel
origin(s).

## Prerequisites

- Oracle Cloud account with Always Free eligibility
- Region with Ampere A1 capacity (retry other regions if “Out of capacity”)
- SSH key pair
- Cloudflare account (free) for a **named** tunnel
- Testnet router ID in `config/deployments/testnet.json` (see Ops below)
- Git + Docker on the VM (bootstrap script installs Docker)

## 1. Create the Always Free VM

1. OCI Console → **Compute → Instances → Create instance**.
2. Image: **Ubuntu 22.04** or **24.04** (aarch64).
3. Shape: **VM.Standard.A1.Flex** (Always Free-eligible). Prefer 4 OCPU / 24 GB;
   if capacity fails, try 2 OCPU / 12 GB or another region.
4. Networking: public subnet; assign a public IP **for SSH only**.
5. Add your SSH public key.
6. Create the instance; note the public IP.

### Security list (SSH only)

Allow ingress TCP **22** from your IP. Do **not** open 80/443 — Cloudflare
Tunnel makes the API public without those ports.

## 2. Bootstrap the VM

From your laptop:

```bash
scp -r scripts/oracle docs/deployment/oracle-always-free.md \
  ubuntu@<PUBLIC_IP>:~/
ssh ubuntu@<PUBLIC_IP>
# Or use the in-repo script after cloning:
sudo bash scripts/oracle/bootstrap-vm.sh
```

`bootstrap-vm.sh` installs Docker Engine + Compose plugin, enables the service,
and adds the current user to the `docker` group (re-login required).

## 3. Deploy the stack

```bash
git clone https://github.com/StellarRoute/StellarRoute.git
cd StellarRoute

cp deploy/env.prod.example .env.prod
# Edit .env.prod — set passwords, ADMIN_AUTH_TOKEN, CORS_ALLOWED_ORIGINS,
# ROUTER_CONTRACT_ADDRESS (from config/deployments/testnet.json).

docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml \
  --env-file .env.prod up -d --build
```

First ARM release build can take **30–90+ minutes**. If the OOM killer strikes,
add swap (e.g. 8G) and retry.

Verify on the VM:

```bash
curl -sf http://127.0.0.1:8080/health && echo OK
curl -sf http://127.0.0.1:8080/health/deps && echo OK
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml \
  --env-file .env.prod ps
```

## 4. Testnet router + pools (before indexer is useful)

On a machine with Soroban CLI (can be the VM or your laptop):

```bash
./scripts/deploy.sh --network testnet
# Commit config/deployments/testnet.json with the new router_contract_id

# Ensure config/pools-testnet.json has real addresses (not PLACEHOLDER_*)
./scripts/register-pools.sh --network testnet
./scripts/verify-pools.sh --network testnet
```

Update `.env.prod` `ROUTER_CONTRACT_ADDRESS`, then:

```bash
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml \
  --env-file .env.prod up -d indexer
```

## 5. Cloudflare Tunnel (public HTTPS)

On the VM (or any host that can reach `127.0.0.1:8080`):

```bash
# Install cloudflared: https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/install-and-setup/installation/
cloudflared tunnel login
cloudflared tunnel create stellarroute-api
cloudflared tunnel route dns stellarroute-api api.<your-domain>
```

Config (`~/.cloudflared/config.yml`):

```yaml
tunnel: <TUNNEL_UUID>
credentials-file: /home/ubuntu/.cloudflared/<TUNNEL_UUID>.json

ingress:
  - hostname: api.<your-domain>
    service: http://127.0.0.1:8080
  - service: http_status:404
```

Run as a service:

```bash
sudo cloudflared service install
sudo systemctl enable --now cloudflared
```

Confirm from the public internet:

```bash
curl -sf https://api.<your-domain>/health
curl -sf https://api.<your-domain>/health/deps
STAGING_API_BASE_URL=https://api.<your-domain> ./scripts/staging-smoke.sh
```

Use a **named** tunnel (not `cloudflared tunnel --url` quick tunnels) so the
hostname stays stable for Vercel env vars.

### No custom domain yet

You can still create a named tunnel and use a `*.cfargotunnel.com` hostname, or
attach a free domain later. Record whatever hostname you get as
`STAGING_API_BASE_URL`.

## 6. Wire Vercel

1. Import `frontend/` (or monorepo root with Root Directory = `frontend`).
2. Set Production env:
   - `NEXT_PUBLIC_API_URL=https://api.<your-domain>` (or `…/api/v1` — both OK)
   - `NEXT_PUBLIC_API_URL_TESTNET` same as above for testnet staging
   - `NEXT_PUBLIC_STELLAR_NETWORK=testnet`
3. Production builds **fail** if the API URL is missing or still `localhost`
   (see `frontend/lib/env-guard.ts`).
4. Update VM `.env.prod` `CORS_ALLOWED_ORIGINS` to include
   `https://www.stellarroute.app`, `https://stellarroute.app`, and
   `https://stellarroute-frontend.vercel.app`; recreate the API container.

Checklist: `docs/deployment/vercel-frontend.md`.

## 7. Smoke + acceptance

```bash
STAGING_API_BASE_URL=https://api.<your-domain> ./scripts/staging-smoke.sh
```

Wave 0 is done when:

- Public `/health` and `/health/deps` succeed
- Public quote returns 200 for a testnet pair with liquidity
- Vercel UI loads quotes from the tunnel URL (not localhost)
- `config/deployments/testnet.json` has a non-empty `router_contract_id`
- No secrets committed

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| OCI “Out of capacity” | Switch region or reduce OCPU/RAM |
| Build OOM killed | Add swap; close other processes; use 24 GB shape |
| API won’t start: CORS | Set non-empty `CORS_ALLOWED_ORIGINS` |
| API won’t start: auth | Set `ADMIN_AUTH_TOKEN`; do not use `ALLOW_INSECURE_PUBLIC_API` |
| Indexer exits immediately | Set valid `ROUTER_CONTRACT_ADDRESS` |
| Browser CORS errors | Add exact origins (`https://www.stellarroute.app`, apex, Vercel) to `CORS_ALLOWED_ORIGINS` and restart API |
| Tunnel 502 | Confirm API listens on `127.0.0.1:8080` (`curl` locally first) |

## Validate compose from a laptop

```bash
cp deploy/env.prod.example /tmp/stellarroute-env.prod
# fill required placeholders in /tmp/stellarroute-env.prod
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml \
  --env-file /tmp/stellarroute-env.prod config >/dev/null && echo OK
```
