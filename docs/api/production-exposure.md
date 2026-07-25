# Production exposure inventory

Endpoint-by-endpoint inventory of what is publicly reachable in local dev vs.
what the API locks down when `STELLARROUTE_ENV=production` (or the
equivalent per-surface override). See
[`docs/development/environment-variables.md`](../development/environment-variables.md#deployment-profile--security-m5)
for the underlying environment variables.

## Legend

- **Public?** — reachable without any credential in the given profile.
- **Auth** — what's required to reach it when not public.
- `AdminAuth` = `ADMIN_AUTH_TOKEN` via `x-admin-token` or `Authorization: Bearer <token>`.
- `API key` = `API_KEYS` via `x-api-key` or `Authorization: Bearer <key>`.

## CORS (issue #1056)

| Surface | Dev/test default | Production default |
|---|---|---|
| Cross-origin requests (all routes) | Any origin (`Access-Control-Allow-Origin: *`) | Only origins listed in `CORS_ALLOWED_ORIGINS`; startup fails if that allowlist is empty |

## Global request auth (issue #1057)

| Surface | Dev/test default | Production default |
|---|---|---|
| All `/api/v1/*` routes not in `PUBLIC_GET_ROUTES` | No API key required (`REQUIRE_AUTH=false`) | API key required (`REQUIRE_AUTH=true`); routes listed in `PUBLIC_GET_ROUTES` remain public for `GET` only |

## Admin / system mutations (issue #1058)

| Endpoint | Method | Public? (dev) | Public? (prod) | Auth |
|---|---|---|---|---|
| `/api/v1/admin/cache/flush/:base/:quote` | POST | No | No | `AdminAuth` |
| `/api/v1/admin/cache/flush` | POST | No | No | `AdminAuth` |
| `/api/v1/admin/kill-switch` | GET | Yes | Yes | — (read-only state) |
| `/api/v1/admin/kill-switch` | POST | No | No | `AdminAuth` |
| `/api/v1/system/canary/report` | GET | Yes | Yes | — (read-only state) |
| `/api/v1/system/canary/config` | POST | No | No | `AdminAuth` |

All of the above deny by default: if `ADMIN_AUTH_TOKEN` isn't configured at
all, every `AdminAuth`-gated route returns `401` regardless of environment —
there is no way to reach them by simply omitting a token.

## Metrics / replay (issue #1059)

| Endpoint | Method | Dev/test default | Production default |
|---|---|---|---|
| `/metrics` (Prometheus) | GET | Public (scrape without auth) | `AdminAuth` required |
| `/metrics/cache` | GET | Public | `AdminAuth` required |
| `/metrics/pool` | GET | Public | `AdminAuth` required |
| `/api/v1/replay` (list) | GET | Public | `AdminAuth` required |
| `/api/v1/replay/:id` (get) | GET | Public | `AdminAuth` required |
| `/api/v1/replay/:id/run` | POST | Public | `AdminAuth` required |
| `/api/v1/replay/:id/diff` | POST | Public | `AdminAuth` required |

Diff from dev defaults: in dev/test, all of the above are open with no
credential so a local Prometheus instance or a developer can scrape
`/metrics` or replay a captured quote without extra setup. Production sets
`STELLARROUTE_ENV=production`, which flips all of these behind the same
`ADMIN_AUTH_TOKEN` used for admin routes — configure your production
Prometheus scraper (or a reverse-proxy scrape-only allowlist) to send
`x-admin-token`.

## Health checks

`/health` and `/health/deps` remain public in every profile — they carry no
sensitive data and load balancers/orchestrators need them reachable without a
credential.
