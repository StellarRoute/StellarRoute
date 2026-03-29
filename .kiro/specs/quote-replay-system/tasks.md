# Implementation Tasks: Deterministic Quote Replay System

## Tasks

- [x] 1. Database migration and core data types
  - [x] 1.1 Create `crates/api/migrations/0002_replay_artifacts.sql` with the `replay_artifacts` table, indexes, and comments
  - [x] 1.2 Create `crates/api/src/replay/mod.rs` with module declarations and public re-exports
  - [x] 1.3 Create `crates/api/src/replay/artifact.rs` with `ReplayArtifact`, `LiquidityCandidate`, `HealthConfigSnapshot`, and `ReplayOutput` structs (full serde support), plus `insert`, `fetch`, `list`, and `prune_older_than` async DB methods using SQLx raw queries matching the migration schema
  - [x] 1.4 Add property-based test `prop_artifact_serde_round_trip` in `artifact.rs` verifying that any `ReplayArtifact` serialises and deserialises without data loss

- [x] 2. Redactor
  - [x] 2.1 Create `crates/api/src/replay/redactor.rs` with `Redactor` struct implementing `redact(&mut ReplayArtifact)` and `redact_value(&mut serde_json::Value)` — recursively replaces all `"asset_issuer"` string values with `"[REDACTED]"`, leaves null/absent fields unchanged
  - [x] 2.2 Add unit test: native-only artifact is unchanged after redaction
  - [x] 2.3 Add unit test: issued asset issuer is replaced with `"[REDACTED]"` in all nested locations
  - [x] 2.4 Add property-based test `prop_redactor_removes_all_issuers` verifying no original issuer string appears in the serialised JSON after redaction

- [x] 3. Replay engine
  - [x] 3.1 Create `crates/api/src/replay/engine.rs` with `ReplayEngine::run(artifact: &ReplayArtifact) -> Result<ReplayOutput>` — pure synchronous function that reconstructs `DirectVenueCandidate` list from `liquidity_snapshot`, sets `now = artifact.captured_at`, runs `FreshnessGuard::evaluate`, `HealthScorer::score_venues`, `GraphFilter::filter_edges`, and `evaluate_single_hop_direct_venues` (re-implemented inline, not imported from `quote.rs`), then returns `ReplayOutput` with `is_deterministic` flag
  - [x] 3.2 Add unit test: hand-crafted artifact with two candidates returns the lower-priced one as `selected_source`
  - [x] 3.3 Add unit test: artifact with `schema_version = 99` returns `ApiError::BadRequest`
  - [x] 3.4 Add unit test: artifact with empty `liquidity_snapshot` returns `ApiError::BadRequest`
  - [x] 3.5 Add property-based test `prop_replay_is_deterministic` verifying that running the engine twice on the same artifact produces identical `selected_source` and `price`

- [x] 4. Diff engine
  - [x] 4.1 Create `crates/api/src/replay/diff.rs` with `DiffEngine::diff(artifact: &ReplayArtifact, replay: &ReplayOutput) -> DiffReport` — compares `price`, `selected_source`, and `path[0].source` fields; numeric strings compared with `1e-7` tolerance; returns `DiffReport { artifact_id, is_identical, divergences }`
  - [x] 4.2 Add unit test: identical original and replay produces `is_identical = true` and empty `divergences`
  - [x] 4.3 Add unit test: differing `price` field produces one `FieldDivergence` with `field = "price"`
  - [x] 4.4 Add unit test: two prices differing by less than `1e-7` are treated as equal
  - [x] 4.5 Add property-based test `prop_diff_reflexive` verifying that `DiffEngine::diff` on a self-consistent artifact always returns `is_identical = true`

- [x] 5. Capture hook
  - [x] 5.1 Create `crates/api/src/replay/capture.rs` with `CaptureHook` struct holding `db: PgPool`, `redactor: Redactor`, and `enabled: bool`; implement `capture(&self, base, quote, params, candidates, response, incident_id)` that builds a `ReplayArtifact`, calls `Redactor::redact`, then spawns a detached `tokio::spawn` task to call `ReplayArtifact::insert` — never awaits the task, logs `WARN` on failure
  - [x] 5.2 Add unit test: when `enabled = false`, no DB insert is attempted
  - [x] 5.3 Add unit test: when `enabled = true`, the spawned task calls insert with a correctly-built artifact

- [x] 6. AppState integration
  - [x] 6.1 Add `pub replay_capture: Option<Arc<CaptureHook>>` field to `AppState` in `crates/api/src/state.rs`; set to `None` in existing constructors; add `AppState::with_replay_capture` constructor that accepts a `CaptureHook`
  - [x] 6.2 Add the single `CaptureHook::capture(...)` call at the end of `get_quote_inner` in `crates/api/src/routes/quote.rs`, after the `QuoteResponse` is fully built, passing the `candidates` vec (cloned as `Vec<LiquidityCandidate>`) and the assembled response

- [x] 7. REST API endpoints
  - [x] 7.1 Create `crates/api/src/routes/replay.rs` with four handlers: `get_artifact` (`GET /{id}`), `run_replay` (`POST /{id}/run`), `diff_replay` (`POST /{id}/diff`), and `list_artifacts` (`GET /`) — all use `Arc<AppState>` and return `Result<Json<T>>` with existing `ApiError` mapping
  - [x] 7.2 Register the four routes in `crates/api/src/routes/mod.rs` under `/api/v1/replay/...`
  - [x] 7.3 Add integration test: `GET /api/v1/replay/{unknown_uuid}` returns HTTP 404
  - [x] 7.4 Add integration test: `POST /api/v1/replay/{id}/run` on a valid artifact returns HTTP 200 with `is_deterministic = true`

- [x] 8. CLI binary
  - [x] 8.1 Create `crates/api/src/bin/replay_cli.rs` with `clap`-based subcommands: `fetch`, `run`, `diff`, `list` — each reads `DATABASE_URL` from env, constructs `ArtifactStore`, calls the appropriate library function, and prints JSON to stdout; exits with code `1` and prints to stderr on any error

- [x] 9. Incident workflow documentation
  - [x] 9.1 Create `docs/incident-replay-workflow.md` documenting: finding `artifact_id` from tracing logs, fetching with CLI and API, running replay, interpreting `DiffReport` for both "no divergence" and "divergence found" cases, and attaching artifacts to incident tickets — include example CLI commands and example JSON output
