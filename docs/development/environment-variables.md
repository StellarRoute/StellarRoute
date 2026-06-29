# Environment Variables Reference

Single authoritative catalog of environment variables used across StellarRoute services. For local setup steps, see [SETUP.md](./SETUP.md).

Copy [`.env.example`](../../.env.example) to `.env` at the repository root and adjust values for your environment.

## Quick reference by service

| Service | Binary / app | Required vars |
|---------|--------------|---------------|
| API | `stellarroute-api` | `DATABASE_URL` |
| Indexer | `stellarroute-indexer` | `DATABASE_URL`, `STELLAR_HORIZON_URL`, `SOROBAN_RPC_URL`, `ROUTER_CONTRACT_ADDRESS` |
| Routing | (library, used by API) | — (all optional) |
| Contracts tooling | `scripts/deploy.sh` | — (`STELLAR_NETWORK` optional) |
| Frontend | Next.js (`frontend/`) | — (all optional) |
| Docker Compose | `docker-compose.yml` | — (defines Postgres/Redis container config) |

---

## Database

PostgreSQL connection strings and pool tuning. Used by the API, indexer, replay CLI, and regional read-replica routing.

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `DATABASE_URL` | string (URL) | API fallback: `postgres://localhost/stellarroute` | **Required** (indexer, replay CLI); validated at API startup | API, Indexer, Replay CLI, Regions | Primary PostgreSQL connection string. Local Docker default: `postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute` |
| `DATABASE_URL_EU_WEST` | string (URL) | — | Optional | API (regions) | Read-replica URL for the `eu-west` region |
| `DATABASE_URL_AP_SOUTHEAST` | string (URL) | — | Optional | API (regions) | Read-replica URL for the `ap-southeast` region |
| `DB_MAX_CONNECTIONS` | integer | `10` | Optional | API | Maximum connections in the API Postgres pool |
| `DB_MIN_CONNECTIONS` | integer | `2` | Optional | API | Minimum idle connections in the API Postgres pool |
| `DB_CONNECTION_TIMEOUT` | integer (seconds) | `30` | Optional | API | Max wait when acquiring a connection from the API pool |
| `DB_IDLE_TIMEOUT` | integer (seconds) | `600` | Optional | API | Close idle API pool connections after this duration |
| `DB_MAX_LIFETIME` | integer (seconds) | `1800` | Optional | API | Recycle API pool connections after this lifetime |
| `DB_STATEMENT_TIMEOUT_MS` | integer (ms) | `5000` | Optional | API | Postgres `statement_timeout` applied on each API pool connection |
| `DB_LOCK_TIMEOUT_MS` | integer (ms) | `2000` | Optional | API | Postgres `lock_timeout` applied on each API pool connection |
| `DB_IDLE_IN_TXN_TIMEOUT_MS` | integer (ms) | `5000` | Optional | API | Postgres `idle_in_transaction_session_timeout` on API pool connections |
| `MAX_CONNECTIONS` | integer | `10` | Optional | Indexer | Maximum connections in the indexer Postgres pool (`config` crate field `max_connections`) |
| `MIN_CONNECTIONS` | integer | `2` | Optional | Indexer | Minimum idle connections in the indexer Postgres pool |
| `CONNECTION_TIMEOUT_SECS` | integer (seconds) | `30` | Optional | Indexer | Max wait when acquiring a connection from the indexer pool |
| `IDLE_TIMEOUT_SECS` | integer (seconds) | `600` | Optional | Indexer | Close idle indexer pool connections after this duration |
| `MAX_LIFETIME_SECS` | integer (seconds) | `1800` | Optional | Indexer | Recycle indexer pool connections after this lifetime |
| `TEST_DATABASE_URL` | string (URL) | — | Optional | API tests | Dedicated database URL for consistency-guard integration tests |

---

## Redis

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `REDIS_URL` | string (URL) | — | Optional | API | Redis connection string for quote caching and rate limiting. When unset, rate limiting falls back to in-memory. Local Docker default: `redis://localhost:6379` |
| `QUOTE_CACHE_TTL_SECONDS` | integer (seconds) | `2` | Optional | API | Time-to-live for cached quote responses in Redis |

---

## Stellar / Horizon

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `STELLAR_HORIZON_URL` | string (URL) | — | **Required** (indexer); optional (API health/lag) | Indexer, API | Stellar Horizon API base URL (e.g. `https://horizon.stellar.org` or `https://horizon-testnet.stellar.org`) |
| `HORIZON_MODE` | string | `poll` | Optional | Indexer | SDEX ingestion mode: `poll` or `sse` |
| `HORIZON_LIMIT` | integer | `200` | Optional | Indexer | Maximum records per Horizon API page request |
| `POLL_INTERVAL_SECS` | integer (seconds) | `2` | Optional | Indexer | Poll interval when Horizon streaming is not used |

---

## Soroban RPC

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `SOROBAN_RPC_URL` | string (URL) | — | **Required** (indexer); optional health check (API) | Indexer, API | Soroban RPC endpoint (e.g. `https://soroban-rpc.testnet.stellar.org`). When set, the API validates reachability at startup if `STARTUP_CREDENTIAL_CHECK=true` |
| `ROUTER_CONTRACT_ADDRESS` | string (Stellar address) | — | **Required** (indexer) | Indexer | Deployed router contract ID for AMM pool discovery |
| `AMM_POOLS` | string (comma-separated) | — | Optional | Indexer | Additional AMM pool addresses to index, appended to DB-discovered pools |

---

## API server

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `API_HOST` | string | `127.0.0.1` | Optional | API | Bind address for the HTTP server |
| `API_PORT` | integer | `3000` | Optional | API | Listen port for the HTTP server |
| `ADMIN_AUTH_TOKEN` | string | — | Optional | API | Bearer token for protected admin/operator endpoints |
| `API_KEYS` | string (comma-separated) | — | Optional | API | Valid API keys for authenticated requests |
| `REQUIRE_AUTH` | boolean | `false` | Optional | API | When `true`, reject requests without a valid API key |
| `STARTUP_CREDENTIAL_CHECK` | boolean | `false` | Optional | API, Indexer | When `true`, verify dependencies (DB, Redis, Horizon, Soroban) are reachable before serving |
| `SHUTDOWN_DRAIN_TIMEOUT_S` | integer (seconds) | `30` | Optional | API, Indexer | Graceful shutdown drain window for in-flight work |
| `RATE_LIMIT_WINDOW_SECS` | integer (seconds) | `60` | Optional | API | Sliding-window length for HTTP rate limiting |
| `RATE_LIMIT_PAIRS` | integer | `60` | Optional | API | Max requests per window for `/api/v1/pairs` |
| `RATE_LIMIT_ORDERBOOK` | integer | `60` | Optional | API | Max requests per window for `/api/v1/orderbook/*` |
| `RATE_LIMIT_QUOTE` | integer | `20` | Optional | API | Max requests per window for `/api/v1/quote/*` |
| `API_V1_SUNSET` | string (HTTP-date) | `Wed, 01 Jul 2026 00:00:00 GMT` | Optional | API | `Sunset` header value for deprecated `/api/v1` routes |
| `API_V1_SUCCESSOR_LINK` | string | `</docs/api/v1-migration-guide>; rel="deprecation"` | Optional | API | `Link` header pointing to the v1 migration guide |
| `IDEMPOTENCY_TTL_SECS` | integer (seconds) | `300` | Optional | API | TTL for idempotent quote deduplication ledger entries |
| `PREWARM_PAIRS` | string (comma-separated) | — | Optional | API | Trading pairs to prewarm in cache (e.g. `native/USDC,native/EURC`) |
| `PREWARM_INTERVAL_SECS` | integer (seconds) | `60` | Optional | API | Interval between cache prewarm cycles |
| `PREWARM_AMOUNT` | string | `1` | Optional | API | Quote amount used during cache prewarm |
| `PREWARM_SLIPPAGE_BPS` | integer | `50` | Optional | API | Slippage (basis points) used during cache prewarm |
| `REPLAY_CAPTURE_ENABLED` | boolean | `false` | Optional | API | Persist quote replay artifacts to Postgres when `true` or `1` |
| `AUDIT_LOG_ENABLED` | boolean | `true` | Optional | API | Enable route audit log writes when not `false`/`0` |
| `ADMIN_AUDIT_ENABLED` | boolean | `true` | Optional | API | Emit admin audit JSON to stdout when not `false`/`0` |
| `LIQUIDITY_THINNESS_ALERT_WEBHOOK_URL` | string (URL) | — | Optional | API | Webhook URL for low-liquidity orderbook alerts |
| `LIQUIDITY_THINNESS_ALERT_THRESHOLDS` | string (JSON) | — | Optional | API | Per-pair depth thresholds for liquidity thinness alerts |
| `HEALTH_SCORE_INTERVAL_SECS` | integer (seconds) | `60` | Optional | API | Interval between venue health score recomputation cycles |
| `HEALTH_SCORE_JITTER_SECS` | integer (seconds) | `10` | Optional | API | Random jitter added to health score scheduler interval |
| `HEALTH_SCORE_MAX_RETRIES` | integer | `3` | Optional | API | Max retries per health score cycle before dead-letter logging |
| `HEALTH_SCORE_RETRY_DELAY_SECS` | integer (seconds) | `5` | Optional | API | Delay between health score retry attempts |

### Quote purger

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `QUOTE_PURGER_ENABLED` | boolean | `true` | Optional | API | Enable automated stale-quote purging |
| `QUOTE_PURGER_INTERVAL_SECS` | integer (seconds) | `3600` | Optional | API | Interval between purge runs |
| `QUOTE_PURGER_REPLAY_RETENTION_DAYS` | integer (days) | `30` | Optional | API | Retention for `replay_artifacts` rows |
| `QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS` | integer (days) | `30` | Optional | API | Retention for `route_audit_log` rows |
| `QUOTE_PURGER_REPLAY_BATCH_SIZE` | integer | `1000` | Optional | API | Max rows deleted per replay-artifacts batch |
| `QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE` | integer | `5000` | Optional | API | Max rows deleted per audit-log batch |
| `QUOTE_PURGER_MAX_ITERATIONS` | integer | `100` | Optional | API | Max delete iterations per purge run |
| `QUOTE_PURGER_PURGE_REPLAY_ARTIFACTS` | boolean | `true` | Optional | API | Purge replay artifacts when `true` |
| `QUOTE_PURGER_PURGE_AUDIT_LOG` | boolean | `true` | Optional | API | Purge audit log rows when `true` |
| `QUOTE_PURGER_LOG_METRICS` | boolean | `true` | Optional | API | Log purge metrics to tracing when `true` |
| `QUOTE_PURGER_SLOW_PURGE_THRESHOLD_SECS` | integer (seconds) | `60` | Optional | API | Alert when a purge run exceeds this duration |
| `QUOTE_PURGER_ALERT_DELETION_THRESHOLD` | integer | `1000000` | Optional | API | Alert when deleted row count exceeds this threshold |

---

## WebSocket

Configured in `crates/api/src/routes/ws/mod.rs`.

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `WS_MAX_CONNECTIONS` | integer | `500` | Optional | API | Maximum concurrent WebSocket connections |
| `WS_POLL_INTERVAL_MS` | integer (ms) | `1000` | Optional | API | Quote broadcaster poll interval |
| `WS_PING_INTERVAL_SECS` | integer (seconds) | `30` | Optional | API | Keepalive ping interval |
| `WS_PONG_TIMEOUT_SECS` | integer (seconds) | `10` | Optional | API | Close connection if pong not received in time |
| `WS_BACKPRESSURE_TIMEOUT_SECS` | integer (seconds) | `10` | Optional | API | Timeout when client send buffer is full |

---

## Indexer

Additional indexer-specific tuning (loaded via the `config` crate from environment unless noted).

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `AMM_POLL_INTERVAL_SECS` | integer (seconds) | `30` | Optional | Indexer | Poll interval for AMM pool updates |
| `STALE_THRESHOLD_SECS` | integer (seconds) | `300` | Optional | Indexer | Mark AMM pools stale after this many seconds without updates |
| `MAINTENANCE_INTERVAL_MINS` | integer (minutes) | `60` | Optional | Indexer | Interval between scheduled maintenance tasks |
| `SNAPSHOT_RETENTION_DAYS` | integer (days) | `90` | Optional | Indexer | Delete snapshots older than this retention period |
| `SNAPSHOT_COMPACTION_HOURS` | integer (hours) | `24` | Optional | Indexer | Compact snapshots older than this threshold |
| `INDEXER_PARTITION_COUNT` | integer | `4` | Optional | Indexer | Number of workload partitions |
| `INDEXER_PARTITION_ID` | integer | `0` | Optional | Indexer | Identifier of this partition instance |
| `HOT_PAIR_ALLOWLIST` | string (comma-separated) | *(empty)* | Optional | Indexer | Hot pair identifiers (e.g. `XLM/USD,USDC/EUR`) |
| `HOT_PAIR_VOLUME_THRESHOLD` | integer | `1000000000` | Optional | Indexer | Volume threshold (native units) to classify a pair as hot |
| `HOT_PAIR_WINDOW_SECS` | integer (seconds) | `300` | Optional | Indexer | Window for hot-pair volume detection |
| `ASSET_METADATA_STALENESS_HOURS` | integer (hours) | `24` | Optional | Indexer | Re-fetch asset metadata after this staleness period |

---

## Routing

Used by the routing library when computing paths inside the API.

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `ROUTING_MAX_HOPS` | integer | `4` | Optional | Routing (API) | Maximum hop depth for multi-hop route discovery |
| `ROUTING_VENUE_ALLOWLIST` | string (comma-separated) | *(empty)* | Optional | Routing (API) | When non-empty, only listed venue types are considered |
| `ROUTING_VENUE_DENYLIST` | string (comma-separated) | *(empty)* | Optional | Routing (API) | Venue types excluded from route discovery |
| `ROUTING_ASSET_DENYLIST` | string (comma-separated) | *(empty)* | Optional | Routing (API) | Asset codes excluded from route discovery |
| `ROUTING_SCORER` | string | `default` | Optional | Routing (API) | Active route scorer name (e.g. `fee_minimizing`) |

---

## Observability / tracing

Shared by API (`crates/api/src/tracing_config.rs`) and indexer (`crates/indexer/src/telemetry.rs`).

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `RUST_LOG` | string (tracing filter) | `info` | Optional | API, Indexer | Log level filter (standard `tracing-subscriber` / `EnvFilter` syntax) |
| `LOG_FORMAT` | string | `pretty` | Optional | API, Indexer | Log output format: `json` or `pretty` |
| `OTEL_SERVICE_NAME` | string | API: `stellarroute`; Indexer: `stellarroute-indexer` | Optional | API, Indexer | Service name attached to OpenTelemetry spans |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | string (URL) | — | Optional | API, Indexer | OTLP collector URL; when unset, trace export is disabled |
| `OTEL_SAMPLING_RATIO` | float (`0.0`–`1.0`) | `1.0` | Optional | API, Indexer | Fraction of traces to sample for export |

---

## Contracts tooling

Used by deployment scripts under `scripts/` (see `scripts/lib/common.sh`).

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `STELLAR_NETWORK` | string | `testnet` | Optional | Deploy scripts | Target network when `--network` is not passed (`testnet` or `mainnet`) |

Deploy scripts also accept CLI flags (`--network`, `--identity`, `--dry-run`) rather than additional environment variables. Network RPC URLs are read from `config/networks.json`.

---

## Frontend

Next.js public variables (prefixed with `NEXT_PUBLIC_` so they are exposed to the browser). See also [frontend/src/FEATURE_FLAGS.md](../../frontend/src/FEATURE_FLAGS.md).

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `NEXT_PUBLIC_API_URL` | string (URL) | `http://localhost:8080/api/v1` | Optional | Frontend | Base URL for REST API requests |
| `NEXT_PUBLIC_FEATURE_ROUTES_BETA` | boolean | `false` | Optional | Frontend | Enable routes beta via `lib/feature-flags.ts` (`true`/`1`/`yes`/`on`) |
| `NEXT_PUBLIC_FLAGS_URL` | string (URL) | — | Optional | Frontend | Remote JSON feature-flag config URL (highest priority) |
| `NEXT_PUBLIC_FLAG_ROUTES_BETA` | boolean | `false` | Optional | Frontend | Enable routes beta via `useFeatureFlag` hook |
| `NEXT_PUBLIC_FLAG_SWAP_UI_V2` | boolean | `false` | Optional | Frontend | Enable swap UI v2 experiment |
| `NEXT_PUBLIC_FLAG_TRANSACTION_HISTORY` | boolean | `false` | Optional | Frontend | Enable transaction history tab |
| `NEXT_PUBLIC_FLAG_ADVANCED_SLIPPAGE` | boolean | `false` | Optional | Frontend | Enable advanced slippage controls |

### Frontend / CI (testing only)

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `CI` | boolean | — | Optional | Playwright | When set, enables CI-specific test settings (retries, GitHub reporter, single worker) |
| `PLAYWRIGHT_BASE_URL` | string (URL) | `http://localhost:3000` | Optional | Playwright | Base URL for end-to-end tests |

---

## Docker Compose

Variables set inside `docker-compose.yml` for local Postgres and Redis containers (not read directly by Rust services except via `DATABASE_URL` / `REDIS_URL` you configure in `.env`).

| Variable | Type | Default | Required | Service(s) | Description |
|----------|------|---------|----------|------------|-------------|
| `POSTGRES_USER` | string | `stellarroute` | — | Docker (Postgres) | Postgres superuser name |
| `POSTGRES_PASSWORD` | string | `stellarroute_dev` | — | Docker (Postgres) | Postgres password |
| `POSTGRES_DB` | string | `stellarroute` | — | Docker (Postgres) | Initial database name |

Postgres is exposed on host port **5432**; Redis on **6379**.

---

## Source map

Variables above were verified against:

- `crates/api/src/bin/stellarroute-api.rs`
- `crates/api/src/routes/ws/mod.rs`
- `crates/api/src/middleware/rate_limit.rs`, `middleware/auth.rs`, `middleware/api_versioning.rs`
- `crates/api/src/tracing_config.rs`, `shutdown.rs`
- `crates/api/src/replay/capture.rs`, `purger/config.rs`
- `crates/api/src/regions/config.rs`, `state.rs`
- `crates/api/src/audit/writer.rs`, `admin_audit.rs`
- `crates/api/src/health_scheduler.rs`, `liquidity_alerts.rs`, `dependency_health.rs`
- `crates/indexer/src/config/mod.rs`, `bin/stellarroute-indexer.rs`
- `crates/indexer/src/telemetry.rs`, `shutdown.rs`, `amm.rs`, `asset_metadata.rs`
- `crates/routing/src/policy.rs`, `scorer.rs`
- `scripts/lib/common.sh`
- `frontend/lib/constants.ts`, `frontend/lib/feature-flags.ts`, `frontend/src/hooks/useFeatureFlag.ts`
- `docker-compose.yml`
