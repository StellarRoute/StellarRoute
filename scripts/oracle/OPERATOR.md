# Oracle Wave 0 — operator steps that require your cloud accounts

Repo automation for Wave 0 is in place. Complete these interactive steps once:

## 1. Oracle Always Free VM (Ops 1)

1. Create an Always Free `VM.Standard.A1.Flex` Ubuntu aarch64 instance (see `docs/deployment/oracle-always-free.md`).
2. SSH in and run:

```bash
git clone https://github.com/StellarRoute/StellarRoute.git
cd StellarRoute
sudo bash scripts/oracle/bootstrap-vm.sh
# re-login for docker group
cp deploy/env.prod.example .env.prod
# Fill secrets; set:
# ROUTER_CONTRACT_ADDRESS=CCOKRYC5ROFYFYDATV2Y3A2DRAGY2LJV7D6B327YFIQS2Y2EWII7TVHS
./scripts/oracle/up.sh --build
```

## 2. Cloudflare Tunnel (Ops 3)

**Preferred (named, stable hostname):**

```bash
TUNNEL_HOSTNAME=api.yourdomain.com ./scripts/oracle/setup-cloudflared.sh
# Then interactive:
cloudflared tunnel login
cloudflared tunnel create stellarroute-api
# edit ~/.cloudflared/config.yml with tunnel UUID
cloudflared tunnel route dns stellarroute-api api.yourdomain.com
sudo cloudflared service install && sudo systemctl enable --now cloudflared
curl -sf https://api.yourdomain.com/health
STAGING_API_BASE_URL=https://api.yourdomain.com ./scripts/staging-smoke.sh
```

**Interim (quick tunnel, hostname rotates on restart):**

```bash
cloudflared tunnel --url http://127.0.0.1:8080
# Copy the https://*.trycloudflare.com URL into Vercel:
#   NEXT_PUBLIC_API_URL / NEXT_PUBLIC_API_URL_TESTNET
# Smoke: STAGING_API_BASE_URL=https://….trycloudflare.com ./scripts/staging-smoke.sh
# Default smoke quote pair is an indexed testnet SDEX book (BTC/EXT); override with
# PROBE_BASE_ASSET / PROBE_QUOTE_ASSET / PROBE_AMOUNT if needed.
```

After the public API URL is known, set GitHub Actions variable/secret `STAGING_API_BASE_URL`
so `.github/workflows/staging-smoke.yml` can run on schedule/dispatch.
## 3. Vercel CORS

Set `CORS_ALLOWED_ORIGINS` on the VM to include the custom domain and Vercel
origins, then recreate the API container:

```env
CORS_ALLOWED_ORIGINS=https://www.stellarroute.app,https://stellarroute.app,https://stellarroute-frontend.vercel.app
```

## Already done in-repo / on-chain / Vercel

- Testnet router: `CCOKRYC5ROFYFYDATV2Y3A2DRAGY2LJV7D6B327YFIQS2Y2EWII7TVHS`
- Adapter + 2 pools registered; `config/deployments/testnet.json` updated
- Compose prod overlay, Oracle runbook, staging smoke, frontend env guard
- Frontend production: https://www.stellarroute.app (also https://stellarroute-frontend.vercel.app)
  (update `NEXT_PUBLIC_API_URL` / `_TESTNET` to the tunnel hostname after Ops 3)

## Local stand-in (Colima) while OCI capacity is pending

If Docker Desktop is unavailable, `colima start --cpu 4 --memory 8` then
`./scripts/oracle/up.sh --build` brings up the same compose stack on localhost:8080.
Use Cloudflare Tunnel against that host for a temporary public API URL.

**Wave 0 live (2026-07-28 Colima stand-in):**
- API: `http://127.0.0.1:8080` + quick tunnel (hostname rotates — check `cloudflared` output)
- Frontend: https://stellarroute-frontend.vercel.app / https://www.stellarroute.app
- Smoke: `STAGING_API_BASE_URL=<tunnel> ./scripts/staging-smoke.sh`
- GitHub Actions var `STAGING_API_BASE_URL` points at the current quick-tunnel URL; update it when you switch to a named tunnel.
- Rebuild API/indexer images after pulling the SDEX heartbeat + AMM retention fixes so `/health` stays green without the SQL keepalive helper.
