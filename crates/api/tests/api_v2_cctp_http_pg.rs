//! Postgres-backed CCTP HTTP concurrency/adversarial tests through the Axum router.
//!
//! Run: `TEST_DATABASE_URL=postgres://... cargo test -p stellarroute-api --test api_v2_cctp_http_pg -- --ignored`

use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use stellarroute_api::{
    cctp::{
        access::{hash_access_token, test_access_token_keyring, TRANSFER_ACCESS_HEADER},
        bootstrap::CctpHttpContext,
        config::CctpConfig,
        gate::{CCTP_CHAIN_KILL_SEPOLIA, CCTP_CHAIN_KILL_STELLAR, REATTEST_MAX_ATTEMPTS},
        idempotency::{canonical_quote_request_hash, PgCctpQuoteIdempotencyStore},
        iris::{IrisClient, IrisFeeQuote, IrisPollOutcome},
        prepare_lock::PgCctpPrepareLockStore,
        readiness::CctpRuntime,
        service::CctpService,
        store::{CctpTransfer, CctpTransferStore, PgCctpTransferStore},
    },
    dependency_health::ExternalDependencyHealth,
    kill_switch::KillSwitchManager,
    models::v2_cctp::{
        CctpChainAsset, CctpDirection, CctpFinality, CctpTransferStatus, CCTP_PROVIDER_ID,
        CCTP_TESTNET_CORRIDOR_ID, SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID,
    },
    state::{AppState, DatabasePools},
};
use stellarroute_routing::health::policy::OverrideDirective;
use tower::ServiceExt;
use uuid::Uuid;

const VALID_STELLAR_RECIPIENT: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
const VALID_EVM_TX_HASH: &str =
    "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

struct CountingIris {
    fee_calls: AtomicUsize,
    poll_calls: AtomicUsize,
    reattest_calls: AtomicUsize,
    fail_reattest: AtomicBool,
}

impl CountingIris {
    fn new() -> Self {
        Self {
            fee_calls: AtomicUsize::new(0),
            poll_calls: AtomicUsize::new(0),
            reattest_calls: AtomicUsize::new(0),
            fail_reattest: AtomicBool::new(false),
        }
    }

    fn set_fail_reattest(&self, fail: bool) {
        self.fail_reattest.store(fail, Ordering::SeqCst);
    }

    fn fee_count(&self) -> usize {
        self.fee_calls.load(Ordering::SeqCst)
    }
    fn poll_count(&self) -> usize {
        self.poll_calls.load(Ordering::SeqCst)
    }
    fn reattest_count(&self) -> usize {
        self.reattest_calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl IrisClient for CountingIris {
    async fn fetch_burn_fees(
        &self,
        _: u32,
        _: u32,
    ) -> Result<IrisFeeQuote, stellarroute_api::cctp::iris::IrisError> {
        self.fee_calls.fetch_add(1, Ordering::SeqCst);
        Ok(IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        })
    }

    async fn poll_messages_by_tx(
        &self,
        _: u32,
        _: &str,
    ) -> Result<IrisPollOutcome, stellarroute_api::cctp::iris::IrisError> {
        self.poll_calls.fetch_add(1, Ordering::SeqCst);
        Ok(IrisPollOutcome::Pending)
    }

    async fn reattest(&self, _: &str) -> Result<(), stellarroute_api::cctp::iris::IrisError> {
        self.reattest_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_reattest.load(Ordering::SeqCst) {
            return Err(stellarroute_api::cctp::iris::IrisError::Http(
                "simulated iris outage".into(),
            ));
        }
        Ok(())
    }
}

fn clear_shell_cctp_env() {
    for key in [
        "CCTP_ENABLED",
        "CCTP_ACCESS_TOKEN_HMAC_KEY",
        "CCTP_STELLAR_RPC_URL",
        "CCTP_SEPOLIA_RPC_URL",
        "SEPOLIA_RPC_URL",
    ] {
        std::env::remove_var(key);
    }
}

fn testnet_cctp_config() -> CctpConfig {
    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
    cfg
}

fn executable_runtime(cfg: &CctpConfig) -> CctpRuntime {
    CctpRuntime::probe_ready_http_harness(cfg, "pg-test-payload-hash")
}

fn sample_quote_body() -> Value {
    json!({
        "corridor_id": CCTP_TESTNET_CORRIDOR_ID,
        "provider": CCTP_PROVIDER_ID,
        "direction": "evm_to_stellar",
        "source_chain_id": SEPOLIA_CHAIN_ID,
        "destination_chain_id": STELLAR_TESTNET_CHAIN_ID,
        "source_asset": CctpChainAsset::sepolia_usdc(),
        "destination_asset": CctpChainAsset::stellar_testnet_usdc(),
        "amount": "100.000000",
        "recipient": VALID_STELLAR_RECIPIENT,
        "mint_submitter": VALID_STELLAR_RECIPIENT,
        "finality": "standard"
    })
}

async fn apply_migrations(pool: &PgPool) {
    for migration in [
        include_str!("../migrations/0015_cctp_transfers.sql"),
        include_str!("../migrations/0016_cctp_transfers_hardening.sql"),
        include_str!("../migrations/0017_cctp_mint_metadata.sql"),
        include_str!("../migrations/0018_cctp_approval_tx_hash.sql"),
        include_str!("../migrations/0019_cctp_approval_verified_at.sql"),
        include_str!("../migrations/20260730_cctp_review_fixes.sql"),
        include_str!("../migrations/20260731_cctp_prepare_lock_hardening.sql"),
        include_str!("../migrations/20260801_cctp_http_gate.sql"),
        include_str!("../migrations/20260802_cctp_http_hardening.sql"),
        include_str!("../migrations/20260803_cctp_reattest_lease.sql"),
    ] {
        for stmt in migration
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            sqlx::query(stmt).execute(pool).await.ok();
        }
    }
}

async fn truncate_cctp(pool: &PgPool) {
    sqlx::query("TRUNCATE cctp_quote_idempotency, cctp_transfers CASCADE")
        .execute(pool)
        .await
        .expect("truncate");
}

struct PgHarness {
    pool: PgPool,
    iris: Arc<CountingIris>,
}

impl PgHarness {
    async fn connect() -> Self {
        clear_shell_cctp_env();
        std::env::set_var("CCTP_IDEMPOTENCY_LEASE_SECS", "2");
        let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL");
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(&url)
            .await
            .expect("connect");
        apply_migrations(&pool).await;
        Self {
            pool,
            iris: Arc::new(CountingIris::new()),
        }
    }

    fn build_context(&self) -> Arc<CctpHttpContext> {
        let mut cfg = testnet_cctp_config();
        cfg.poll_interval_secs = 60;

        let store: Arc<dyn CctpTransferStore> =
            Arc::new(PgCctpTransferStore::new(self.pool.clone()));
        let idempotency = Arc::new(PgCctpQuoteIdempotencyStore::new(self.pool.clone()));
        let runtime = executable_runtime(&cfg);
        let service = Arc::new(CctpService {
            config: cfg.clone(),
            store,
            prepare_lock: Arc::new(PgCctpPrepareLockStore::new(self.pool.clone())),
            iris: self.iris.clone(),
            kill_switch: Arc::new(KillSwitchManager::new(None)),
            runtime: runtime.clone(),
        });
        Arc::new(CctpHttpContext {
            config: cfg,
            service,
            runtime,
            idempotency,
            access_token_keys: test_access_token_keyring(),
        })
    }

    async fn fresh_router(&self) -> axum::Router {
        self.fresh_router_with(None, None).await
    }

    async fn fresh_router_with(
        &self,
        dependency_health: Option<Arc<ExternalDependencyHealth>>,
        kill_switch: Option<Arc<KillSwitchManager>>,
    ) -> axum::Router {
        let mut state = AppState::new(DatabasePools::new(self.pool.clone(), None))
            .with_cctp(self.build_context());
        state.external_dependency_health = dependency_health
            .unwrap_or_else(|| Arc::new(ExternalDependencyHealth::new(vec![], vec![])));
        if let Some(ks) = kill_switch {
            state.kill_switch = ks;
        }
        stellarroute_api::routes::create_router(state.into_arc())
    }
}

async fn post_quote(
    router: &axum::Router,
    body: &Value,
    idempotency_key: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v2/bridge/cctp/quote")
        .header("content-type", "application/json");
    if let Some(key) = idempotency_key {
        builder = builder.header("idempotency-key", key);
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

async fn get_transfer(
    router: &axum::Router,
    transfer_id: &str,
    access_token: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().uri(format!("/api/v2/bridge/cctp/{transfer_id}"));
    if let Some(token) = access_token {
        builder = builder.header(TRANSFER_ACCESS_HEADER, token);
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

async fn post_reattest(
    router: &axum::Router,
    transfer_id: &str,
    access_token: &str,
) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v2/bridge/cctp/{transfer_id}/reattest"))
                .header(TRANSFER_ACCESS_HEADER, access_token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

fn quote_request_hash(body: &Value) -> String {
    canonical_quote_request_hash(body).expect("hash")
}

async fn seed_transfer_row(pool: &PgPool, transfer: &CctpTransfer) {
    let store = PgCctpTransferStore::new(pool.clone());
    let mut base = transfer.clone();
    let status = base.status;
    base.status = CctpTransferStatus::Created;
    base.source_tx_hash = None;
    base.message_nonce = None;
    store.insert(&base).await.unwrap();
    sqlx::query(
        r#"
        UPDATE cctp_transfers
        SET source_tx_hash = $2,
            message_nonce = $3,
            status = $4,
            updated_at = $5,
            retry_count = $6
        WHERE transfer_id = $1
        "#,
    )
    .bind(transfer.transfer_id)
    .bind(&transfer.source_tx_hash)
    .bind(&transfer.message_nonce)
    .bind(status_str_for_test(status))
    .bind(transfer.updated_at)
    .bind(transfer.retry_count as i32)
    .execute(pool)
    .await
    .unwrap();
}

fn status_str_for_test(status: CctpTransferStatus) -> &'static str {
    match status {
        CctpTransferStatus::AwaitingAttestation => "awaiting_attestation",
        CctpTransferStatus::AttestationFailed => "attestation_failed",
        CctpTransferStatus::Created => "created",
        _ => "created",
    }
}

fn sample_pg_transfer(id: Uuid, access_hash: &str) -> CctpTransfer {
    let now = Utc::now();
    CctpTransfer {
        transfer_id: id,
        support_reference_id: format!("pg-http-{id}"),
        corridor_id: CCTP_TESTNET_CORRIDOR_ID.into(),
        provider: CCTP_PROVIDER_ID.into(),
        direction: CctpDirection::EvmToStellar,
        source_chain_id: SEPOLIA_CHAIN_ID.into(),
        destination_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
        source_asset: "erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238".into(),
        source_asset_canonical: format!("{SEPOLIA_CHAIN_ID}/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"),
        destination_asset: "erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA".into(),
        destination_asset_canonical: format!(
            "{STELLAR_TESTNET_CHAIN_ID}/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA"
        ),
        sender: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        recipient: VALID_STELLAR_RECIPIENT.into(),
        mint_submitter: Some(VALID_STELLAR_RECIPIENT.into()),
        amount: "100.000000".into(),
        destination_amount: "100.000000".into(),
        finality: CctpFinality::Standard,
        runtime_fee_quote: Some("1".into()),
        max_fee: Some("1".into()),
        fee_expires_at: Some(now + Duration::minutes(10)),
        quote_expires_at: now + Duration::minutes(10),
        status: CctpTransferStatus::AwaitingAttestation,
        source_tx_hash: Some(VALID_EVM_TX_HASH.into()),
        source_approval_tx_hash: None,
        source_approval_verified_at: None,
        destination_tx_hash: None,
        iris_message_hash: None,
        message_nonce: None,
        raw_message: None,
        attestation: None,
        retry_count: 0,
        last_provider_error: None,
        last_provider_code: None,
        version: 1,
        created_at: now,
        updated_at: now,
        terminal_at: None,
        mint_payload_hash: None,
        mint_payload_expires_at: None,
        approval_payload_hash: None,
        approval_expiration_ledger: None,
        burn_payload_hash: None,
        burn_prepare_step: None,
        access_token_hash: Some(access_hash.into()),
        last_polled_at: None,
        poll_lease_until: None,
        reattest_lease_owner_hash: None,
        reattest_lease_until: None,
        reattest_attempt_count: 0,
        reattest_cooldown_until: None,
    }
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_concurrent_idempotent_quote_converges() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;

    let body = sample_quote_body();
    let key = "pg-idem-race-1";
    let router = Arc::new(harness.fresh_router().await);
    let n = 8usize;
    let mut handles = Vec::new();
    for _ in 0..n {
        let router = router.clone();
        let body = body.clone();
        handles.push(tokio::spawn(async move {
            let mut attempt = 0;
            loop {
                let (status, resp) = post_quote(&router, &body, Some(key)).await;
                if status == StatusCode::OK {
                    return (status, resp);
                }
                if (status == StatusCode::TOO_EARLY
                    || status == StatusCode::TOO_MANY_REQUESTS
                    || status == StatusCode::SERVICE_UNAVAILABLE)
                    && attempt < 80
                {
                    attempt += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    continue;
                }
                return (status, resp);
            }
        }));
    }

    let mut successes = Vec::new();
    for h in handles {
        let (status, resp) = h.await.unwrap();
        if status == StatusCode::OK {
            successes.push(resp);
        }
    }
    assert!(
        !successes.is_empty(),
        "at least one caller must converge to 200"
    );

    let transfer_id = successes[0]["data"]["transfer_id"]
        .as_str()
        .unwrap()
        .to_string();
    let access_token = successes[0]["data"]["access_token"].as_str().unwrap();

    for resp in &successes[1..] {
        assert_eq!(resp["data"]["transfer_id"].as_str().unwrap(), transfer_id);
        assert_eq!(resp["data"]["access_token"].as_str().unwrap(), access_token);
    }

    let transfer_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cctp_transfers")
        .fetch_one(&harness.pool)
        .await
        .unwrap();
    let completed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cctp_quote_idempotency WHERE idempotency_key = $1 AND state = 'completed'",
    )
    .bind(key)
    .fetch_one(&harness.pool)
    .await
    .unwrap();

    assert_eq!(transfer_count, 1, "exactly one transfer row");
    assert_eq!(completed, 1, "exactly one completed idempotency claim");
    assert_eq!(harness.iris.fee_count(), 1, "one Iris fee fetch");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_idempotency_conflict_returns_409_without_extra_transfer() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;
    let router = harness.fresh_router().await;

    let body_a = sample_quote_body();
    let mut body_b = body_a.clone();
    body_b["amount"] = json!("101.000000");

    let (s1, _) = post_quote(&router, &body_a, Some("conflict-key")).await;
    assert_eq!(s1, StatusCode::OK);

    let (s2, resp2) = post_quote(&router, &body_b, Some("conflict-key")).await;
    assert_eq!(s2, StatusCode::CONFLICT);
    assert!(
        resp2["data"]["status"]
            .as_str()
            .unwrap_or("")
            .contains("idempotency")
            || resp2["data"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("Idempotency")
    );

    let transfer_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cctp_transfers")
        .fetch_one(&harness.pool)
        .await
        .unwrap();
    assert_eq!(transfer_count, 1);
    assert_eq!(harness.iris.fee_count(), 1);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_lease_takeover_after_crash_reuses_transfer_id_and_token() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;

    let body = sample_quote_body();
    let key = "lease-takeover";
    let request_hash = quote_request_hash(&body);
    let transfer_id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::minutes(10);

    sqlx::query(
        r#"
        INSERT INTO cctp_quote_idempotency
            (idempotency_key, request_hash, transfer_id, state, lease_owner_hash, lease_expires_at, expires_at)
        VALUES ($1, $2, $3, 'pending', 'dead-owner', NOW() - INTERVAL '1 minute', $4)
        "#,
    )
    .bind(key)
    .bind(&request_hash)
    .bind(transfer_id)
    .bind(expires_at)
    .execute(&harness.pool)
    .await
    .unwrap();

    let router = harness.fresh_router().await;
    let (status, resp) = post_quote(&router, &body, Some(key)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        resp["data"]["transfer_id"].as_str().unwrap(),
        transfer_id.to_string()
    );

    let ring = test_access_token_keyring();
    let expected_token = ring.derive_idempotent_token(key, &request_hash, transfer_id);
    assert_eq!(
        resp["data"]["access_token"].as_str().unwrap(),
        expected_token
    );

    let transfer_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cctp_transfers")
        .fetch_one(&harness.pool)
        .await
        .unwrap();
    assert_eq!(transfer_count, 1);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_finalize_failure_then_retry_recovers_without_orphan_completed() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;

    let body = sample_quote_body();
    let key = "finalize-retry";
    let request_hash = quote_request_hash(&body);
    let transfer_id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::minutes(10);
    let access_hash = hash_access_token("placeholder-not-used");

    sqlx::query(
        r#"
        INSERT INTO cctp_quote_idempotency
            (idempotency_key, request_hash, transfer_id, state, lease_owner_hash, lease_expires_at, expires_at)
        VALUES ($1, $2, $3, 'pending', 'owner-a', NOW() - INTERVAL '1 minute', $4)
        "#,
    )
    .bind(key)
    .bind(&request_hash)
    .bind(transfer_id)
    .bind(expires_at)
    .execute(&harness.pool)
    .await
    .unwrap();

    let store = PgCctpTransferStore::new(harness.pool.clone());
    let mut blocking = sample_pg_transfer(transfer_id, &access_hash);
    blocking.status = CctpTransferStatus::Created;
    store.insert(&blocking).await.unwrap();

    let router = harness.fresh_router().await;
    let (fail_status, fail_body) = post_quote(&router, &body, Some(key)).await;
    assert!(
        fail_status.is_server_error() || fail_status == StatusCode::TOO_EARLY,
        "finalize must not succeed with duplicate transfer, got {fail_status} {:?}",
        fail_body["data"]
    );

    let completed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cctp_quote_idempotency WHERE idempotency_key = $1 AND state = 'completed'",
    )
    .bind(key)
    .fetch_one(&harness.pool)
    .await
    .unwrap();
    assert_eq!(completed, 0, "no completed idempotency on failed finalize");

    sqlx::query("DELETE FROM cctp_transfers WHERE transfer_id = $1")
        .bind(transfer_id)
        .execute(&harness.pool)
        .await
        .unwrap();

    sqlx::query(
        "UPDATE cctp_quote_idempotency SET lease_expires_at = NOW() - INTERVAL '1 second' WHERE idempotency_key = $1",
    )
    .bind(key)
    .execute(&harness.pool)
    .await
    .unwrap();

    let (ok_status, resp) = post_quote(&router, &body, Some(key)).await;
    assert_eq!(ok_status, StatusCode::OK);
    assert_eq!(
        resp["data"]["transfer_id"].as_str().unwrap(),
        transfer_id.to_string()
    );

    let transfer_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cctp_transfers")
        .fetch_one(&harness.pool)
        .await
        .unwrap();
    assert_eq!(transfer_count, 1);
    assert_eq!(completed_after_retry(&harness.pool, key).await, 1);
}

async fn completed_after_retry(pool: &PgPool, key: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM cctp_quote_idempotency WHERE idempotency_key = $1 AND state = 'completed'",
    )
    .bind(key)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_db_contains_no_plaintext_secrets() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;
    let router = harness.fresh_router().await;

    let body = sample_quote_body();
    let (status, resp) = post_quote(&router, &body, Some("secret-scan")).await;
    assert_eq!(status, StatusCode::OK);
    let token = resp["data"]["access_token"].as_str().unwrap();

    let dump: String = sqlx::query_scalar(
        r#"
        SELECT COALESCE(string_agg(row_to_json(t)::text, ' '), '')
        FROM (
            SELECT transfer_id::text, access_token_hash, raw_message, attestation, iris_message_hash
            FROM cctp_transfers
            UNION ALL
            SELECT idempotency_key, request_hash, transfer_id::text, state, lease_owner_hash
            FROM cctp_quote_idempotency
        ) t
        "#,
    )
    .fetch_one(&harness.pool)
    .await
    .unwrap_or_default();

    assert!(!dump.contains(token), "plaintext access token in DB");
    assert!(
        !dump.to_ascii_lowercase().contains("response_json"),
        "idempotency must not store response_json column"
    );
    assert!(!dump.contains("AAAA"), "no raw XDR persisted from builders");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_uniform_404_for_missing_and_wrong_token() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;
    let router = harness.fresh_router().await;

    let missing_id = Uuid::new_v4();
    let (s1, b1) = get_transfer(&router, &missing_id.to_string(), Some("any-token")).await;
    assert_eq!(s1, StatusCode::NOT_FOUND);

    let body = sample_quote_body();
    let (_, resp) = post_quote(&router, &body, None).await;
    let transfer_id = resp["data"]["transfer_id"].as_str().unwrap();

    let (s2, b2) = get_transfer(&router, transfer_id, Some("wrong-token-value")).await;
    assert_eq!(s2, StatusCode::NOT_FOUND);

    let (s3, b3) = get_transfer(&router, transfer_id, None).await;
    assert_eq!(s3, StatusCode::NOT_FOUND);

    assert_eq!(b1["data"]["error"], b2["data"]["error"]);
    assert_eq!(b2["data"]["error"], b3["data"]["error"]);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_concurrent_poll_acquires_single_iris_call_per_interval() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;

    let transfer_id = Uuid::new_v4();
    let token = "poll-test-token-value-1234567890";
    let access_hash = hash_access_token(token);
    seed_transfer_row(
        &harness.pool,
        &sample_pg_transfer(transfer_id, &access_hash),
    )
    .await;

    let router = harness.fresh_router().await;
    let mut handles = Vec::new();
    for _ in 0..8 {
        let router = router.clone();
        let tid = transfer_id.to_string();
        let token = token.to_string();
        handles.push(tokio::spawn(async move {
            get_transfer(&router, &tid, Some(&token)).await
        }));
    }
    for h in handles {
        let (status, body) = h.await.unwrap();
        assert_eq!(
            status,
            StatusCode::OK,
            "poll GET failed: {:?}",
            body["data"]
        );
    }
    assert_eq!(
        harness.iris.poll_count(),
        1,
        "one Iris poll per interval under concurrent GETs"
    );

    sqlx::query(
        "UPDATE cctp_transfers SET poll_lease_until = NOW() - INTERVAL '1 second', last_polled_at = NOW() - INTERVAL '2 minutes' WHERE transfer_id = $1",
    )
    .bind(transfer_id)
    .execute(&harness.pool)
    .await
    .unwrap();

    let (status, _) = get_transfer(&router, &transfer_id.to_string(), Some(token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        harness.iris.poll_count(),
        2,
        "second poll allowed after lease expiry and interval"
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_concurrent_reattest_single_claim_and_iris_call() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;

    let transfer_id = Uuid::new_v4();
    let token = "reattest-token-value-abcdefghijklmnop";
    let access_hash = hash_access_token(token);
    let mut transfer = sample_pg_transfer(transfer_id, &access_hash);
    transfer.status = CctpTransferStatus::AttestationFailed;
    transfer.message_nonce = Some("nonce-reattest-1".into());
    transfer.updated_at = Utc::now() - Duration::seconds(120);
    seed_transfer_row(&harness.pool, &transfer).await;

    let router = harness.fresh_router().await;
    let mut handles = Vec::new();
    for _ in 0..6 {
        let router = router.clone();
        let tid = transfer_id.to_string();
        let token = token.to_string();
        handles.push(tokio::spawn(async move {
            post_reattest(&router, &tid, &token).await
        }));
    }

    let mut ok = 0;
    let mut denied = 0;
    for h in handles {
        let (status, _) = h.await.unwrap();
        if status == StatusCode::OK {
            ok += 1;
        } else {
            denied += 1;
        }
    }
    assert_eq!(ok, 1, "exactly one reattest claim");
    assert!(denied >= 1, "other callers rejected");
    assert_eq!(harness.iris.reattest_count(), 1);

    let retry_count: i32 =
        sqlx::query_scalar("SELECT retry_count FROM cctp_transfers WHERE transfer_id = $1")
            .bind(transfer_id)
            .fetch_one(&harness.pool)
            .await
            .unwrap();
    assert_eq!(retry_count, 1);

    let attempt_count: i32 = sqlx::query_scalar(
        "SELECT reattest_attempt_count FROM cctp_transfers WHERE transfer_id = $1",
    )
    .bind(transfer_id)
    .fetch_one(&harness.pool)
    .await
    .unwrap();
    assert_eq!(attempt_count, 1);

    let status_row: String =
        sqlx::query_scalar("SELECT status FROM cctp_transfers WHERE transfer_id = $1")
            .bind(transfer_id)
            .fetch_one(&harness.pool)
            .await
            .unwrap();
    assert_eq!(status_row, "awaiting_attestation");

    let router2 = harness.fresh_router().await;
    let (cooldown_status, _) = post_reattest(&router2, &transfer_id.to_string(), token).await;
    assert_eq!(cooldown_status, StatusCode::BAD_REQUEST);
}

async fn set_chain_kill(kill: &KillSwitchManager, venue: &str) {
    let mut state = kill.get_state().await;
    state
        .venues
        .insert(venue.to_string(), OverrideDirective::ForceExclude);
    kill.update_state(state).await.unwrap();
}

async fn get_v2(router: &axum::Router) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_reattest_iris_failure_keeps_failed_state_and_cooldown() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;
    harness.iris.set_fail_reattest(true);

    let transfer_id = Uuid::new_v4();
    let token = "reattest-fail-token-abcdefghijklmnop";
    let access_hash = hash_access_token(token);
    let mut transfer = sample_pg_transfer(transfer_id, &access_hash);
    transfer.status = CctpTransferStatus::AttestationFailed;
    transfer.message_nonce = Some("nonce-fail-1".into());
    seed_transfer_row(&harness.pool, &transfer).await;

    let router = harness.fresh_router().await;
    let (status, _) = post_reattest(&router, &transfer_id.to_string(), token).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(harness.iris.reattest_count(), 1);

    let row: (String, i32, i32, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
        "SELECT status, retry_count, reattest_attempt_count, reattest_cooldown_until FROM cctp_transfers WHERE transfer_id = $1",
    )
    .bind(transfer_id)
    .fetch_one(&harness.pool)
    .await
    .unwrap();
    assert_eq!(row.0, "attestation_failed");
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 1);
    assert!(row.3.is_some());

    let (retry_status, _) = post_reattest(&router, &transfer_id.to_string(), token).await;
    assert_eq!(retry_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        harness.iris.reattest_count(),
        1,
        "cooldown blocks second Iris call"
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_reattest_lease_expiry_allows_recovery_without_corrupting_nonce() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;

    let transfer_id = Uuid::new_v4();
    let token = "reattest-lease-token-abcdefghijklmnop";
    let access_hash = hash_access_token(token);
    let nonce = "nonce-lease-expiry-1";
    let mut transfer = sample_pg_transfer(transfer_id, &access_hash);
    transfer.status = CctpTransferStatus::AttestationFailed;
    transfer.message_nonce = Some(nonce.into());
    seed_transfer_row(&harness.pool, &transfer).await;

    sqlx::query(
        r#"
        UPDATE cctp_transfers
        SET reattest_lease_owner_hash = 'stale-owner',
            reattest_lease_until = NOW() - INTERVAL '1 second'
        WHERE transfer_id = $1
        "#,
    )
    .bind(transfer_id)
    .execute(&harness.pool)
    .await
    .unwrap();

    let router = harness.fresh_router().await;
    let (status, _) = post_reattest(&router, &transfer_id.to_string(), token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(harness.iris.reattest_count(), 1);

    let stored_nonce: String =
        sqlx::query_scalar("SELECT message_nonce FROM cctp_transfers WHERE transfer_id = $1")
            .bind(transfer_id)
            .fetch_one(&harness.pool)
            .await
            .unwrap();
    assert_eq!(stored_nonce, nonce);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_reattest_attempt_cap_blocks_further_provider_calls() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;

    let transfer_id = Uuid::new_v4();
    let token = "reattest-cap-token-abcdefghijklmnop";
    let access_hash = hash_access_token(token);
    let mut transfer = sample_pg_transfer(transfer_id, &access_hash);
    transfer.status = CctpTransferStatus::AttestationFailed;
    transfer.message_nonce = Some("nonce-cap".into());
    transfer.reattest_attempt_count = REATTEST_MAX_ATTEMPTS;
    seed_transfer_row(&harness.pool, &transfer).await;

    sqlx::query("UPDATE cctp_transfers SET reattest_attempt_count = $2 WHERE transfer_id = $1")
        .bind(transfer_id)
        .bind(REATTEST_MAX_ATTEMPTS as i32)
        .execute(&harness.pool)
        .await
        .unwrap();

    let router = harness.fresh_router().await;
    let (status, _) = post_reattest(&router, &transfer_id.to_string(), token).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(harness.iris.reattest_count(), 0);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_symmetric_chain_and_dependency_gates_block_without_iris() {
    let harness = PgHarness::connect().await;
    truncate_cctp(&harness.pool).await;

    let health = Arc::new(ExternalDependencyHealth::new(vec![], vec![]));
    let kill = Arc::new(KillSwitchManager::new(None));
    let router = harness
        .fresh_router_with(Some(health.clone()), Some(kill.clone()))
        .await;

    let body = sample_quote_body();
    let baseline = post_quote(&router, &body, Some("gate-baseline")).await;
    assert_ne!(harness.iris.fee_count(), usize::MAX);

    set_chain_kill(&kill, CCTP_CHAIN_KILL_SEPOLIA).await;
    let router2 = harness
        .fresh_router_with(Some(health.clone()), Some(kill.clone()))
        .await;
    let fees_before = harness.iris.fee_count();
    let (status, _) = post_quote(&router2, &body, Some("gate-sepolia-kill")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(harness.iris.fee_count(), fees_before);

    let kill2 = Arc::new(KillSwitchManager::new(None));
    set_chain_kill(&kill2, CCTP_CHAIN_KILL_STELLAR).await;
    let router3 = harness
        .fresh_router_with(Some(health.clone()), Some(kill2))
        .await;
    let fees_before2 = harness.iris.fee_count();
    let (status2, _) = post_quote(&router3, &body, Some("gate-stellar-kill")).await;
    assert_eq!(status2, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(harness.iris.fee_count(), fees_before2);

    let health3 = Arc::new(ExternalDependencyHealth::new(vec![], vec![]));
    for _ in 0..3 {
        health3.record_evm_rpc_result(false);
    }
    let router4 = harness
        .fresh_router_with(
            Some(health3.clone()),
            Some(Arc::new(KillSwitchManager::new(None))),
        )
        .await;
    let fees_before3 = harness.iris.fee_count();
    let (status3, _) = post_quote(&router4, &body, Some("gate-evm-unhealthy")).await;
    assert_eq!(status3, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(harness.iris.fee_count(), fees_before3);

    let (_, v2) = get_v2(&router4).await;
    let corridors = &v2["data"]["supported_corridors"];
    assert!(corridors
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["executable"] == false));

    let _ = baseline;
}
