# Frontend

Next.js App Router UI for StellarRoute.

## Local development

```bash
cp .env.example .env.local
npm install
npm run dev
```

`NEXT_PUBLIC_API_URL` may point at `http://localhost:8080` in development.

## Production / Vercel

See [`docs/deployment/vercel-frontend.md`](../docs/deployment/vercel-frontend.md).

Cross-chain CCTP (Stellar ↔ Sepolia) is signed-live proven both ways on testnet;
see [`docs/cctp/signed-live-stellar-to-sepolia.md`](../docs/cctp/signed-live-stellar-to-sepolia.md)
and [`docs/cctp/signed-live-sepolia-to-stellar.md`](../docs/cctp/signed-live-sepolia-to-stellar.md).
Public API enablement remains fail-closed until operators set `CCTP_ENABLED`.

Production builds enforce a public API URL via `lib/env-guard.ts` when
`VERCEL_ENV=production` or `STELLARROUTE_ENV=production`.

```bash
npm run test -- lib/env-guard.test.ts
VERCEL_ENV=production \
NEXT_PUBLIC_API_URL=https://api.example.com \
NEXT_PUBLIC_STELLAR_NETWORK=testnet \
npm run build
```
