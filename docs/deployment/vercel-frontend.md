# Vercel frontend deploy checklist (Wave 0 / issue #1036)

Import the `frontend/` app on [Vercel](https://vercel.com) (Hobby is fine for Wave 0).

## Project settings

| Setting | Value |
|---------|--------|
| Framework Preset | Next.js |
| Root Directory | `frontend` (monorepo) |
| Build Command | `npm run build` (default from `frontend/vercel.json`) |
| Install Command | `npm install` |
| Output | `.next` |
| Node | 20.x (recommended) |

## Environment variables

### Production

| Variable | Example | Required |
|----------|---------|----------|
| `NEXT_PUBLIC_API_URL` | `https://api.your-tunnel-host` | **Yes** (or per-network URL below) |
| `NEXT_PUBLIC_API_URL_TESTNET` | same as API for testnet staging | Recommended |
| `NEXT_PUBLIC_STELLAR_NETWORK` | `testnet` | **Yes** for Wave 0 |
| `STELLARROUTE_ENV` | `production` | Optional; also enforced via `VERCEL_ENV=production` |

Production builds **fail** if the API URL is missing or points at `localhost`
(`frontend/lib/env-guard.ts`).

### Preview

| Variable | Notes |
|----------|--------|
| `NEXT_PUBLIC_API_URL` / `_TESTNET` | Point at staging API, or leave unset (preview is not `VERCEL_ENV=production` so localhost guard does not fire — still prefer a real staging URL) |
| `NEXT_PUBLIC_STELLAR_NETWORK` | `testnet` |

### Development (local)

Localhost API is allowed. Copy `frontend/.env.example` → `.env.local`:

```env
NEXT_PUBLIC_API_URL=http://localhost:8080/api/v1
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_FLAG_SWAP_UI_V2=true
```

## CORS on the API

After the Vercel production URL / custom domain is known, set on the Oracle VM
`.env.prod`:

```env
CORS_ALLOWED_ORIGINS=https://www.stellarroute.app,https://stellarroute.app,https://stellarroute-frontend.vercel.app
```

Recreate the API container, then hard-refresh the site.

## Verify

```bash
# Unit tests for the env guard
npm --prefix frontend run test -- src/../lib/env-guard.test.ts

# Production-like build with a public API URL (must succeed)
VERCEL_ENV=production \
NEXT_PUBLIC_API_URL=https://api.example.com \
NEXT_PUBLIC_STELLAR_NETWORK=testnet \
npm --prefix frontend run build

# Must fail (localhost)
VERCEL_ENV=production \
NEXT_PUBLIC_API_URL=http://localhost:8080 \
npm --prefix frontend run build
# → [env-guard] …
```

## Domains

1. Vercel → Project → Settings → Domains — production custom domain is `https://www.stellarroute.app` (also allow apex `https://stellarroute.app` in CORS).
2. Keep `https://stellarroute-frontend.vercel.app` in `CORS_ALLOWED_ORIGINS` alongside the custom domain.
